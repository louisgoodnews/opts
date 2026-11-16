//! Unit tests of `lib.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::discover::discover;
use super::*;
use std::fs;
use std::time::Duration;

mod discovery;
mod references;

fn fixture() -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("opts_cat_{}_{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Vehicles/Bus1/model")).unwrap();
    fs::create_dir_all(dir.join("maps/Town")).unwrap();
    fs::write(
        dir.join("Vehicles/Bus1/Bus1.bus"),
        "[friendlyname]\nACME\nCity 1\nRed\n\n[model]\nmodel\\model.cfg\n\n\
             [passengercabin]\nmodel\\cabin.cfg\n\n\
             [add_camera_driver]\n0.5\n0.3\n1.2\n0\n60\n0\n0\n",
    )
    .unwrap();
    fs::write(
        dir.join("Vehicles/Bus1/model/cabin.cfg"),
        "[drivpos]\n0.5\n0.3\n1.0\n0\n",
    )
    .unwrap();
    fs::write(
        dir.join("Vehicles/Bus1/Bus1_trail.bus"),
        "[friendlyname]\nACME\nTrailer\nRed\n",
    )
    .unwrap();
    fs::write(dir.join("Vehicles/Bus1/model/model.cfg"), "x").unwrap();
    fs::write(dir.join("maps/Town/global.cfg"), "[name]\nTown\n").unwrap();
    dir
}


#[test]
fn staleness_tracks_age_and_root() {
    let root = fixture();
    let db = root.join("cat.db");
    let cat = Catalogue::open(&db).unwrap();
    let day = Duration::from_secs(24 * 3600);
    assert_eq!(cat.staleness(&root, day).unwrap(), Staleness::Missing);
    discover(&db, &root, &|_| {}).unwrap();
    assert!(matches!(
        cat.staleness(&root, day).unwrap(),
        Staleness::Fresh { .. }
    ));
    assert!(matches!(
        cat.staleness(&root, Duration::ZERO).unwrap(),
        Staleness::Fresh { .. } | Staleness::Expired { .. }
    ));
    assert_eq!(
        cat.staleness(Path::new("/elsewhere"), day).unwrap(),
        Staleness::RootChanged
    );
    cat.with(|c| {
        c.execute(
            "UPDATE meta SET value='1' WHERE key='last_scan_completed_at'",
            [],
        )
    })
    .unwrap();
    assert!(matches!(
        cat.staleness(&root, day).unwrap(),
        Staleness::Expired { .. }
    ));
    let _ = fs::remove_dir_all(&root);
}


