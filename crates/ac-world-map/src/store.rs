//! The tiers on disk: each in its own folder under the map folder, its
//! parts as they were downloaded and `manifest.json`, written last, saying
//! it is whole. A tier is installed when its manifest reads and every part
//! it names is there at its size.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use crate::Tier;
use crate::manifest::{Manifest, Part};
use crate::part::read_table;

/// The marker's name in a tier's folder.
pub const MARKER: &str = "manifest.json";

/// A tier's folder.
#[must_use]
pub fn folder(root: &Path, tier: Tier) -> PathBuf {
    root.join(tier.name())
}

/// The tier's manifest, when it is installed whole.
#[must_use]
pub fn installed(root: &Path, tier: Tier) -> Option<Manifest> {
    let dir = folder(root, tier);
    let bytes = fs::read(dir.join(MARKER)).ok()?;
    let manifest = Manifest::read(&bytes).ok()?;
    manifest
        .parts
        .iter()
        .all(|p| fs::metadata(dir.join(&p.file)).is_ok_and(|m| m.len() == p.bytes))
        .then_some(manifest)
}

/// Whether the file at `path` is `part`: its size and SHA-256.
#[must_use]
pub fn holds(path: &Path, part: &Part) -> bool {
    if !fs::metadata(path).is_ok_and(|m| m.len() == part.bytes) {
        return false;
    }
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hash.update(&buf[..n]),
            Err(_) => return false,
        }
    }
    hex(&hash.finalize()) == part.sha256
}

/// Lower-case hex of `bytes`.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Writes the marker: the tier is whole from now on.
///
/// # Errors
/// The file system's.
pub fn write_marker(root: &Path, tier: Tier, manifest_bytes: &[u8]) -> io::Result<()> {
    let dir = folder(root, tier);
    let temp = dir.join("manifest.json.new");
    {
        let mut f = File::create(&temp)?;
        f.write_all(manifest_bytes)?;
        f.sync_all()?;
    }
    fs::rename(temp, dir.join(MARKER))
}

/// Takes the marker away: the tier is no longer drawn.
///
/// # Errors
/// The file system's, other than the marker not being there.
pub fn unmark(root: &Path, tier: Tier) -> io::Result<()> {
    match fs::remove_file(folder(root, tier).join(MARKER)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Deletes the tier: its marker first, then its folder.
///
/// # Errors
/// The file system's.
pub fn remove(root: &Path, tier: Tier) -> io::Result<()> {
    unmark(root, tier)?;
    match fs::remove_dir_all(folder(root, tier)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Deletes what is in the tier's folder that `manifest` does not name.
///
/// # Errors
/// The file system's.
pub fn tidy(root: &Path, tier: Tier, manifest: &Manifest) -> io::Result<()> {
    let dir = folder(root, tier);
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name != MARKER
            && !manifest.parts.iter().any(|p| p.file == name)
            && entry.file_type()?.is_file()
        {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

/// Where a file is: its part and place in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct At {
    part: usize,
    offset: u64,
    len: u32,
}

/// An installed tier, to read tiles from. The parts' tables are read the
/// first time a tile is.
#[derive(Debug)]
pub struct TierFiles {
    dir: PathBuf,
    manifest: Manifest,
    table: OnceLock<HashMap<String, At>>,
}

impl TierFiles {
    /// The tier in `dir`, as `manifest` says.
    #[must_use]
    pub fn new(dir: PathBuf, manifest: Manifest) -> Self {
        Self {
            dir,
            manifest,
            table: OnceLock::new(),
        }
    }

    /// Its manifest.
    #[must_use]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn table(&self) -> &HashMap<String, At> {
        self.table.get_or_init(|| {
            let mut table = HashMap::new();
            for (i, p) in self.manifest.parts.iter().enumerate() {
                let Ok(file) = File::open(self.dir.join(&p.file)) else {
                    continue;
                };
                let Ok(entries) = read_table(BufReader::new(file), p.bytes) else {
                    continue;
                };
                for e in entries {
                    table.insert(
                        e.path,
                        At {
                            part: i,
                            offset: e.offset,
                            len: e.len,
                        },
                    );
                }
            }
            table
        })
    }

    /// The bytes of the file at `path`.
    #[must_use]
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        let at = *self.table().get(path)?;
        let part = self.manifest.parts.get(at.part)?;
        let mut file = File::open(self.dir.join(&part.file)).ok()?;
        file.seek(SeekFrom::Start(at.offset)).ok()?;
        let mut bytes = vec![0u8; at.len as usize];
        file.read_exact(&mut bytes).ok()?;
        Some(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::tests::one_part;
    use crate::part::write;

    pub(crate) fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ac-world-map-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn an_installed_tier_reads_its_tiles_from_its_part_and_goes_whole() {
        let root = temp_root("store");
        let part = write(&[
            ("webp/world/0/0_0.webp", b"world"),
            ("webp/towns/8/28_3.webp", b"town!"),
        ]);
        let sha = hex(&Sha256::digest(&part));
        let manifest = one_part(1, part.len(), &sha);
        let dir = folder(&root, Tier::Standard);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("atlas-map-standard-1.bin"), &part).unwrap();
        fs::write(dir.join("stray.bin"), b"x").unwrap();
        // Not installed until the marker is written.
        assert!(installed(&root, Tier::Standard).is_none());
        write_marker(&root, Tier::Standard, manifest.as_bytes()).unwrap();
        let m = installed(&root, Tier::Standard).unwrap();
        assert!(holds(&dir.join(&m.parts[0].file), &m.parts[0]));
        tidy(&root, Tier::Standard, &m).unwrap();
        assert!(!dir.join("stray.bin").exists());
        let files = TierFiles::new(dir.clone(), m);
        assert_eq!(
            files.read("webp/towns/8/28_3.webp").as_deref(),
            Some(&b"town!"[..])
        );
        assert_eq!(
            files.read("webp/world/0/0_0.webp").as_deref(),
            Some(&b"world"[..])
        );
        assert_eq!(files.read("webp/none.webp"), None);
        // A part cut short: not installed.
        fs::write(dir.join("atlas-map-standard-1.bin"), &part[..10]).unwrap();
        assert!(installed(&root, Tier::Standard).is_none());
        remove(&root, Tier::Standard).unwrap();
        assert!(!dir.exists());
        let _ = fs::remove_dir_all(root);
    }
}
