//! The real packs, when `AC_WORLD_MAP_PACKS` names the folder holding
//! `<tier>-r1/` as the bake tool writes them; skipped otherwise.
use ac_world_map::Tier;
use ac_world_map::manifest::Manifest;
use ac_world_map::store::TierFiles;

#[test]
fn every_real_tier_reads_and_its_tiles_decode() {
    let Ok(dir) = std::env::var("AC_WORLD_MAP_PACKS") else {
        return;
    };
    for tier in Tier::ALL {
        let folder = std::path::Path::new(&dir).join(format!("{}-r1", tier.name()));
        let Ok(bytes) = std::fs::read(folder.join(tier.manifest_file())) else {
            continue;
        };
        let m = Manifest::read(&bytes).unwrap();
        let index = m.index.clone();
        let files = TierFiles::new(folder, m);
        let mut n = 0;
        for (i, l) in index.world.iter().enumerate().rev().take(3) {
            for r in 0..l.rows.min(4) {
                for c in 0..l.cols.min(4) {
                    let path = l.path(c, r).unwrap();
                    let px = ac_world_map::service::decode(&files.read(&path).unwrap())
                        .unwrap_or_else(|| panic!("{path} decodes"));
                    assert_eq!(
                        (px.width, px.height),
                        (index.tile_size, index.tile_size),
                        "{i} {path}"
                    );
                    n += 1;
                }
            }
        }
        for l in &index.towns {
            for r in 0..l.rows {
                for c in 0..l.cols {
                    if let Some(path) = l.path(c, r) {
                        assert!(files.read(&path).is_some(), "{path}");
                        n += 1;
                    }
                }
            }
        }
        eprintln!(
            "{}: {} world levels, {} town levels, {n} tiles read",
            tier.name(),
            index.world.len(),
            index.towns.len()
        );
    }
}

/// The Standard tier from the real release, when `AC_WORLD_MAP_LIVE` is
/// set (it downloads about 11 MB).
#[test]
fn the_standard_tier_downloads_from_the_release() {
    use ac_world_map::WorldMaps;
    if std::env::var_os("AC_WORLD_MAP_LIVE").is_none() {
        return;
    }
    let root = std::env::temp_dir().join(format!("ac-world-map-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut maps =
        ac_world_map::service::MapService::start(root.clone(), ac_world_map::DEFAULT_SOURCE)
            .unwrap();
    maps.ask();
    maps.download(Tier::Standard);
    let start = std::time::Instant::now();
    loop {
        let s = maps.status();
        if s.busy.is_none() && (s.tier(Tier::Standard).installed.is_some() || s.failed.is_some()) {
            break;
        }
        assert!(start.elapsed().as_secs() < 300, "{s:?}");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let s = maps.status();
    eprintln!("{s:?} in {:?}", start.elapsed());
    assert_eq!(s.failed, None);
    assert!(
        s.tiers.iter().all(|t| t.offered.is_some()),
        "every tier asked: {s:?}"
    );
    assert!(maps.index(Tier::Standard).is_some());
    let _ = std::fs::remove_dir_all(root);
}
