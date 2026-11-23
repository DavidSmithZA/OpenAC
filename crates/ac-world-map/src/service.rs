//! The detailed map's service: one thread asks the release about the
//! tiers, downloads and removes them, one at a time; two more read and
//! decode the tiles a window wants. A window sees it as [`WorldMaps`].
//!
//! A download takes the tier's marker away first (the tier stops being
//! drawn), keeps every part already on disk that matches the manifest,
//! fetches the rest (each checked for its size, SHA-256 and table before
//! it takes its name), then writes the marker and deletes what the
//! manifest does not name. Stopped part way, what is on disk stays for the
//! next try.

use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, mpsc};
use std::time::Duration;

use ac_launcher_core::cancel::CancelToken;
use ac_launcher_core::net::{
    HttpTransport, body_read_error, read_bounded, send_with_validated_redirects,
};
use ac_launcher_core::updates::UpdateError;
use ac_launcher_core::url::WebUrl;
use sha2::{Digest, Sha256};

use crate::manifest::{self, Manifest, Part};
use crate::part::read_table;
use crate::store::{self, TierFiles};
use crate::{Busy, Loaded, MapIndex, MapsStatus, Pixels, Tier, TileKey, WorldMaps};

/// How long one part may take to come.
const PART_TIMEOUT: Duration = Duration::from_secs(600);
/// How long a manifest may take.
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Threads reading and decoding tiles.
const TILE_THREADS: usize = 2;

/// What the asking thread does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Job {
    Ask,
    Download(Tier),
    Remove(Tier),
}

#[derive(Debug)]
struct State {
    status: MapsStatus,
    revision: u64,
    files: HashMap<Tier, Arc<TierFiles>>,
    indexes: HashMap<Tier, Arc<MapIndex>>,
    cancel: Option<CancelToken>,
    asked: bool,
}

impl State {
    fn changed(&mut self) {
        self.revision += 1;
    }

    fn install(&mut self, tier: Tier, files: TierFiles) {
        let m = files.manifest();
        self.status.tiers[tier as usize].installed = Some(m.release);
        self.indexes.insert(tier, Arc::new(m.index.clone()));
        self.files.insert(tier, Arc::new(files));
    }

    fn uninstall(&mut self, tier: Tier) {
        self.status.tiers[tier as usize].installed = None;
        self.indexes.remove(&tier);
        self.files.remove(&tier);
    }
}

#[derive(Debug, Default)]
struct TileInner {
    wanted: VecDeque<(Tier, TileKey)>,
    done: Vec<Loaded>,
    closed: bool,
}

#[derive(Debug, Default)]
struct TileQueue {
    inner: Mutex<TileInner>,
    wake: Condvar,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The service.
#[derive(Debug)]
pub struct MapService {
    state: Arc<Mutex<State>>,
    jobs: mpsc::Sender<Job>,
    tiles: Arc<TileQueue>,
}

impl MapService {
    /// The service for the tiers under `root`, published at `source` (a
    /// folder URL), over the operating system's HTTPS.
    ///
    /// # Errors
    /// The system's TLS library cannot be set up.
    pub fn start(root: PathBuf, source: &str) -> Result<Self, String> {
        let transport = ac_launcher_net::OsHttpTransport::new().map_err(|e| e.to_string())?;
        Ok(Self::with_transport(root, source, Arc::new(transport)))
    }

    /// The service over `transport`.
    #[must_use]
    pub fn with_transport(root: PathBuf, source: &str, transport: Arc<dyn HttpTransport>) -> Self {
        let mut state = State {
            status: MapsStatus::empty(),
            revision: 1,
            files: HashMap::new(),
            indexes: HashMap::new(),
            cancel: None,
            asked: false,
        };
        for tier in Tier::ALL {
            if let Some(m) = store::installed(&root, tier) {
                state.install(tier, TierFiles::new(store::folder(&root, tier), m));
            }
        }
        let state = Arc::new(Mutex::new(state));
        let (jobs, rx) = mpsc::channel();
        let mut source = source.to_owned();
        if !source.ends_with('/') {
            source.push('/');
        }
        let worker = Worker {
            root,
            source,
            transport,
            state: Arc::clone(&state),
        };
        let _ = std::thread::Builder::new()
            .name("world-map".into())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    worker.run(job);
                }
            });
        let tiles = Arc::new(TileQueue::default());
        for i in 0..TILE_THREADS {
            let (tiles, state) = (Arc::clone(&tiles), Arc::clone(&state));
            let _ = std::thread::Builder::new()
                .name(format!("world-map-tiles-{i}"))
                .spawn(move || read_tiles(&tiles, &state));
        }
        Self { state, jobs, tiles }
    }
}

impl Drop for MapService {
    fn drop(&mut self) {
        if let Some(c) = &lock(&self.state).cancel {
            c.cancel();
        }
        lock(&self.tiles.inner).closed = true;
        self.tiles.wake.notify_all();
    }
}

impl WorldMaps for MapService {
    fn status(&self) -> MapsStatus {
        lock(&self.state).status.clone()
    }

    fn revision(&self) -> u64 {
        lock(&self.state).revision
    }

    fn ask(&mut self) {
        let mut s = lock(&self.state);
        if s.asked {
            return;
        }
        s.asked = true;
        drop(s);
        let _ = self.jobs.send(Job::Ask);
    }

    fn download(&mut self, tier: Tier) {
        {
            let mut s = lock(&self.state);
            if matches!(s.status.busy, Some(Busy::Downloading { .. })) {
                return;
            }
            // Shown at once, before the thread takes it up.
            s.status.busy = Some(Busy::Downloading {
                tier,
                done: 0,
                total: s.status.tier(tier).bytes(),
            });
            s.status.failed = None;
            s.changed();
        }
        let _ = self.jobs.send(Job::Download(tier));
    }

    fn cancel(&mut self) {
        if let Some(c) = &lock(&self.state).cancel {
            c.cancel();
        }
    }

    fn remove(&mut self, tier: Tier) {
        let _ = self.jobs.send(Job::Remove(tier));
    }

    fn index(&self, tier: Tier) -> Option<Arc<MapIndex>> {
        lock(&self.state).indexes.get(&tier).cloned()
    }

    fn want(&mut self, tier: Tier, keys: &[TileKey]) {
        let mut q = lock(&self.tiles.inner);
        q.wanted.clear();
        q.wanted.extend(keys.iter().map(|k| (tier, *k)));
        drop(q);
        self.tiles.wake.notify_all();
    }

    fn take_loaded(&mut self) -> Vec<Loaded> {
        std::mem::take(&mut lock(&self.tiles.inner).done)
    }
}

fn read_tiles(tiles: &TileQueue, state: &Mutex<State>) {
    loop {
        let (tier, key) = {
            let mut q = lock(&tiles.inner);
            loop {
                if q.closed {
                    return;
                }
                if let Some(next) = q.wanted.pop_front() {
                    break next;
                }
                q = tiles
                    .wake
                    .wait(q)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };
        let files = lock(state).files.get(&tier).cloned();
        let pixels = files.and_then(|f| {
            let path = f.manifest().index.path(key)?;
            decode(&f.read(&path)?)
        });
        lock(&tiles.inner).done.push(Loaded { tier, key, pixels });
    }
}

/// A WebP as RGBA pixels.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Pixels> {
    let mut decoder = image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).ok()?;
    let (width, height) = decoder.dimensions();
    let mut pixels = vec![0; decoder.output_buffer_size()?];
    decoder.read_image(&mut pixels).ok()?;
    let rgba = if decoder.has_alpha() {
        pixels
    } else {
        pixels
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect()
    };
    Some(Pixels {
        width,
        height,
        rgba,
    })
}

struct Worker {
    root: PathBuf,
    source: String,
    transport: Arc<dyn HttpTransport>,
    state: Arc<Mutex<State>>,
}

impl Worker {
    fn set(&self, f: impl FnOnce(&mut State)) {
        let mut s = lock(&self.state);
        f(&mut s);
        s.changed();
    }

    fn url(&self, file: &str) -> Result<WebUrl, UpdateError> {
        WebUrl::parse(&format!("{}{file}", self.source))
            .ok_or_else(|| UpdateError::failed("The map's address is not a web address."))
    }

    fn run(&self, job: Job) {
        match job {
            Job::Ask => self.ask(),
            Job::Download(tier) => self.download(tier),
            Job::Remove(tier) => self.remove(tier),
        }
    }

    fn manifest(
        &self,
        tier: Tier,
        cancel: &CancelToken,
    ) -> Result<(Manifest, Vec<u8>), UpdateError> {
        let url = self.url(&tier.manifest_file())?;
        let mut response = send_with_validated_redirects(
            &*self.transport,
            &url,
            "map manifest",
            Some(MANIFEST_TIMEOUT),
            |_| {},
            cancel,
        )?;
        if response.status != 200 {
            return Err(UpdateError::failed(format!(
                "The {} map is not published (the server answered {}).",
                tier.words(),
                response.status
            )));
        }
        let bytes = read_bounded(
            &mut response,
            manifest::MAX_BYTES as u64,
            "The map's manifest is too large.",
            cancel,
        )?;
        let m = Manifest::read(&bytes).map_err(|e| UpdateError::failed(format!("{e}.")))?;
        Ok((m, bytes))
    }

    fn ask(&self) {
        self.set(|s| s.status.busy = Some(Busy::Asking));
        for tier in Tier::ALL {
            if let Ok((m, _)) = self.manifest(tier, &CancelToken::none()) {
                self.set(|s| s.status.tiers[tier as usize].offered = Some((m.release, m.bytes())));
            }
        }
        self.set(|s| {
            if s.status.busy == Some(Busy::Asking) {
                s.status.busy = None;
            }
        });
    }

    fn remove(&self, tier: Tier) {
        self.set(|s| {
            s.uninstall(tier);
            s.status.busy = Some(Busy::Removing(tier));
        });
        let result = store::remove(&self.root, tier);
        self.set(|s| {
            s.status.busy = None;
            s.status.failed = result
                .err()
                .map(|e| format!("The {} map could not be deleted: {e}.", tier.words()));
        });
    }

    fn download(&self, tier: Tier) {
        let cancel = CancelToken::new();
        self.set(|s| {
            s.cancel = Some(cancel.clone());
            s.status.busy = Some(Busy::Downloading {
                tier,
                done: 0,
                total: s.status.tier(tier).bytes(),
            });
            s.status.failed = None;
        });
        let result = self.fetch_tier(tier, &cancel);
        self.set(|s| {
            s.cancel = None;
            s.status.busy = None;
            s.status.failed = match result {
                Ok(()) | Err(UpdateError::Cancelled) => None,
                Err(e) => Some(format!("The {} map did not download: {e}", tier.words())),
            };
        });
    }

    fn fetch_tier(&self, tier: Tier, cancel: &CancelToken) -> Result<(), UpdateError> {
        let (manifest, bytes) = self.manifest(tier, cancel)?;
        let total = manifest.bytes();
        self.set(|s| {
            s.status.tiers[tier as usize].offered = Some((manifest.release, total));
        });
        let dir = store::folder(&self.root, tier);
        let io = |what: &str| {
            let what = what.to_owned();
            move |e: std::io::Error| UpdateError::Failed {
                message: format!("{what}: {e}."),
                io: None,
            }
        };
        // Up to date already.
        if store::installed(&self.root, tier).is_some_and(|m| m == manifest) {
            return Ok(());
        }
        // Not drawn while it changes.
        self.set(|s| s.uninstall(tier));
        store::unmark(&self.root, tier).map_err(io("The map's folder"))?;
        fs::create_dir_all(&dir).map_err(io("The map's folder"))?;
        let mut done = 0;
        for part in &manifest.parts {
            let dest = dir.join(&part.file);
            if !store::holds(&dest, part) {
                let temp = dir.join(format!("{}.download", part.file));
                self.fetch_part(part, &temp, done, total, tier, cancel)?;
                fs::rename(&temp, &dest).map_err(io("A map part"))?;
            }
            done += part.bytes;
            self.progress(tier, done, total);
        }
        store::write_marker(&self.root, tier, &bytes).map_err(io("The map's marker"))?;
        store::tidy(&self.root, tier, &manifest).map_err(io("The map's folder"))?;
        let files = TierFiles::new(dir, manifest);
        self.set(|s| s.install(tier, files));
        Ok(())
    }

    fn progress(&self, tier: Tier, done: u64, total: u64) {
        self.set(|s| s.status.busy = Some(Busy::Downloading { tier, done, total }));
    }

    fn fetch_part(
        &self,
        part: &Part,
        temp: &Path,
        before: u64,
        total: u64,
        tier: Tier,
        cancel: &CancelToken,
    ) -> Result<(), UpdateError> {
        let url = self.url(&part.file)?;
        let mut response = send_with_validated_redirects(
            &*self.transport,
            &url,
            "map part",
            Some(PART_TIMEOUT),
            |_| {},
            cancel,
        )?;
        if response.status != 200 {
            return Err(UpdateError::failed(format!(
                "A part was not there (the server answered {}).",
                response.status
            )));
        }
        let wrong = || UpdateError::failed("A part was not what the manifest says.");
        if response.content_length.is_some_and(|l| l != part.bytes) {
            return Err(wrong());
        }
        let mut file = File::create(temp).map_err(|e| UpdateError::Failed {
            message: format!("A map part could not be written: {e}."),
            io: None,
        })?;
        let mut hash = Sha256::new();
        let mut got: u64 = 0;
        let mut buf = vec![0u8; 64 * 1024];
        let mut last_told = 0;
        loop {
            cancel.check().map_err(|_| UpdateError::Cancelled)?;
            let n = response
                .body
                .read(&mut buf)
                .map_err(|e| UpdateError::Transport(body_read_error(e, cancel)))?;
            if n == 0 {
                break;
            }
            got += n as u64;
            if got > part.bytes {
                return Err(wrong());
            }
            hash.update(&buf[..n]);
            file.write_all(&buf[..n]).map_err(|e| UpdateError::Failed {
                message: format!("A map part could not be written: {e}."),
                io: None,
            })?;
            if got - last_told >= 512 * 1024 {
                last_told = got;
                self.progress(tier, before + got, total);
            }
        }
        drop(file);
        if got != part.bytes || store::hex(&hash.finalize()) != part.sha256 {
            let _ = fs::remove_file(temp);
            return Err(wrong());
        }
        let table = File::open(temp).map(BufReader::new);
        if table.map_or(true, |t| read_table(t, part.bytes).is_err()) {
            let _ = fs::remove_file(temp);
            return Err(wrong());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::tests::one_part;
    use crate::part::write;
    use ac_launcher_core::net::{HttpError, HttpRequest, HttpResponse};
    use std::time::Instant;

    /// Serves fixed bytes by URL; one address redirects to another.
    #[derive(Debug, Default)]
    struct Fake {
        files: Mutex<HashMap<String, Vec<u8>>>,
        asked: Mutex<Vec<String>>,
    }

    impl HttpTransport for Fake {
        fn get(&self, request: &HttpRequest, _: &CancelToken) -> Result<HttpResponse, HttpError> {
            let url = request.url.to_string();
            lock(&self.asked).push(url.clone());
            let (status, location, body) = if url.ends_with(".bin") && !url.contains("assets") {
                (302, Some(url.replace("/rel/", "/assets/")), Vec::new())
            } else {
                match lock(&self.files).get(&url) {
                    Some(b) => (200, None, b.clone()),
                    None => (404, None, Vec::new()),
                }
            };
            Ok(HttpResponse {
                status,
                location,
                content_length: Some(body.len() as u64),
                content_encoding: Vec::new(),
                requested_url: request.url.clone(),
                body: Box::new(std::io::Cursor::new(body)),
            })
        }
    }

    fn webp_tile() -> Vec<u8> {
        let mut out = Vec::new();
        image_webp::WebPEncoder::new(&mut out)
            .encode(&[10, 20, 30, 255], 1, 1, image_webp::ColorType::Rgba8)
            .unwrap();
        out
    }

    fn until(what: &str, mut f: impl FnMut() -> bool) {
        let start = Instant::now();
        while !f() {
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "waited for {what}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn a_tier_downloads_through_a_redirect_draws_its_tiles_and_is_removed() {
        let root =
            std::env::temp_dir().join(format!("ac-world-map-service-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let tile = webp_tile();
        let part = write(&[
            ("webp/world/0/0_0.webp", tile.as_slice()),
            ("webp/towns/8/28_3.webp", b"not a picture"),
        ]);
        let sha = store::hex(&Sha256::digest(&part));
        let fake = Arc::new(Fake::default());
        {
            let mut f = lock(&fake.files);
            f.insert(
                "https://example.test/rel/atlas-map-standard.json".into(),
                one_part(1, part.len(), &sha).into_bytes(),
            );
            f.insert(
                "https://example.test/assets/atlas-map-standard-1.bin".into(),
                part.clone(),
            );
        }
        let mut maps =
            MapService::with_transport(root.clone(), "https://example.test/rel", fake.clone());
        assert_eq!(maps.status().tier(Tier::Standard).installed, None);
        maps.download(Tier::Standard);
        until("the download", || {
            let s = maps.status();
            s.busy.is_none() && s.tier(Tier::Standard).installed.is_some()
        });
        assert_eq!(maps.status().failed, None);
        assert_eq!(
            maps.status().tier(Tier::Standard).offered,
            Some((1, part.len() as u64))
        );
        let index = maps.index(Tier::Standard).unwrap();
        let world = TileKey {
            level: crate::LevelRef::World(0),
            c: 0,
            r: 0,
        };
        let town = TileKey {
            level: crate::LevelRef::Town(0),
            c: 28,
            r: 3,
        };
        assert!(index.path(world).is_some());
        maps.want(Tier::Standard, &[world, town]);
        let mut loaded = Vec::new();
        until("the tiles", || {
            loaded.extend(maps.take_loaded());
            loaded.len() == 2
        });
        let w = loaded.iter().find(|l| l.key == world).unwrap();
        assert_eq!(
            w.pixels.as_ref().map(|p| (p.width, p.height, p.rgba.len())),
            Some((1, 1, 4))
        );
        // Not a picture: back with nothing.
        assert!(
            loaded
                .iter()
                .find(|l| l.key == town)
                .unwrap()
                .pixels
                .is_none()
        );
        // A second service finds it installed.
        let again =
            MapService::with_transport(root.clone(), "https://example.test/rel", fake.clone());
        assert_eq!(again.status().tier(Tier::Standard).installed, Some(1));
        drop(again);
        // Downloading again fetches only the manifest: the part is there.
        let before = lock(&fake.asked).len();
        maps.download(Tier::Standard);
        until("the second download", || maps.status().busy.is_none());
        assert_eq!(lock(&fake.asked).len(), before + 1);
        maps.remove(Tier::Standard);
        until("the removal", || {
            let s = maps.status();
            s.busy.is_none() && s.tier(Tier::Standard).installed.is_none()
        });
        assert!(!store::folder(&root, Tier::Standard).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_part_not_as_the_manifest_says_fails_and_installs_nothing() {
        let root = std::env::temp_dir().join(format!("ac-world-map-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let part = write(&[("webp/world/0/0_0.webp", b"x")]);
        let fake = Arc::new(Fake::default());
        {
            let mut f = lock(&fake.files);
            f.insert(
                "https://example.test/rel/atlas-map-standard.json".into(),
                one_part(1, part.len(), &"00".repeat(32)).into_bytes(),
            );
            f.insert(
                "https://example.test/assets/atlas-map-standard-1.bin".into(),
                part,
            );
        }
        let mut maps = MapService::with_transport(root.clone(), "https://example.test/rel/", fake);
        maps.download(Tier::Standard);
        until("the failure", || maps.status().failed.is_some());
        let s = maps.status();
        assert_eq!(s.tier(Tier::Standard).installed, None);
        assert!(s.failed.unwrap().contains("not what the manifest says"));
        // The Hd manifest is not published: asking leaves it unknown.
        maps.ask();
        until("the asking", || maps.status().busy.is_none());
        assert_eq!(maps.status().tier(Tier::Hd).offered, None);
        assert!(maps.status().tier(Tier::Standard).offered.is_some());
        let _ = fs::remove_dir_all(root);
    }
}
