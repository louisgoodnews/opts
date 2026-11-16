//! Test support: builds a miniature OMSI installation on disk, runs the real discovery on it
//! and returns the menu lists. Used by integration tests so they never depend on a local
//! OMSI installation.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::{load_menu_lists, Catalogue, GameCatalogue};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn entry(id: u32, code: u32, name: &str) -> String {
    format!("{id}\n{code}\n0\n1.0\n2.0\n3.0\n0.0\n0.0\n0.0\n1.0\n18\n{name}\n")
}

/// 42 maps (`Berlin-Spandau`, `Grundorf` with entry points, plus 40 generic ones) and 60
/// playable vehicles (`MAN_SD77` among them).
#[must_use]
pub fn fixture_lists() -> GameCatalogue {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("opts_fixture_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let spandau = format!(
        "[name]\nBerlin-Spandau\n\n[entrypoints]\n2\n{}{}",
        entry(1, 100, "Spandau Westerwaldstr./Siegener Str."),
        entry(2, 101, "Rathaus Spandau"),
    );
    let grundorf = format!(
        "[name]\nGrundorf\n\n[entrypoints]\n1\n{}",
        entry(1, 200, "Bauernhof")
    );
    for (folder, cfg) in [("Berlin-Spandau", spandau), ("Grundorf", grundorf)] {
        let dir = root.join("maps").join(folder);
        std::fs::create_dir_all(&dir).expect("fixture dir");
        std::fs::write(dir.join("global.cfg"), cfg).expect("fixture cfg");
    }
    for i in 0..40 {
        let dir = root.join("maps").join(format!("Generic_Map_{i:02}"));
        std::fs::create_dir_all(&dir).expect("fixture dir");
        std::fs::write(
            dir.join("global.cfg"),
            format!("[name]\nGeneric Map {i:02}\n"),
        )
        .expect("fixture cfg");
    }
    let mut vehicles: Vec<(String, String, String)> = vec![
        ("MAN_SD77".into(), "MAN SD200".into(), "SD77".into()),
        ("MAN_EN92_main".into(), "MAN".into(), "NL202 - EN92".into()),
    ];
    vehicles.extend((0..58).map(|i| {
        (
            format!("Fleet_{i:02}"),
            "Fleet".into(),
            format!("Model {i:02}"),
        )
    }));
    for (id, maker, model) in vehicles
        .iter()
        .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
    {
        let dir = root.join("Vehicles").join(id);
        std::fs::create_dir_all(&dir).expect("fixture dir");
        std::fs::write(
            dir.join(format!("{id}.bus")),
            format!(
                "[friendlyname]\n{maker}\n{model}\nStandard\n\n[passengercabin]\ncabin.cfg\n\n\
                 [add_camera_driver]\n0.5\n0.3\n1.2\n0\n60\n0\n0\n"
            ),
        )
        .expect("fixture bus");
        std::fs::write(dir.join("cabin.cfg"), "[drivpos]\n0.5\n0.3\n1.0\n0\n")
            .expect("fixture cabin");
    }
    let db = root.join("catalogue.db");
    opts_catalogue::discover(&db, &root, &|_| {}).expect("fixture discovery");
    let lists = load_menu_lists(&Catalogue::open(&db).expect("fixture open"));
    let _ = std::fs::remove_dir_all(&root);
    lists
}
