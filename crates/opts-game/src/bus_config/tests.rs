//! Unit tests of `bus_config`.

use std::collections::BTreeMap;

use super::*;

fn catalogue() -> GameCatalogue {
    GameCatalogue {
        maps: Vec::new(),
        vehicles: vec![
            VehicleItem::named("citaro", "Mercedes", "Citaro"),
            VehicleItem::named("lions", "MAN", "Lion's City"),
        ],
    }
}

/// Repaints Default and Red; the option `Klima` is a switch (0 or 1), `Sitze` a choice of 1 to 3.
fn repaints(_: &VehicleItem) -> VehicleRepaints {
    let mut setvars = BTreeMap::new();
    setvars.insert(
        "Default".to_string(),
        vec![
            ("Klima".to_string(), "0".to_string()),
            ("Sitze".to_string(), "1".to_string()),
        ],
    );
    setvars.insert(
        "Red".to_string(),
        vec![
            ("Klima".to_string(), "1".to_string()),
            ("Sitze".to_string(), "3".to_string()),
        ],
    );
    VehicleRepaints {
        default_name: "Default".into(),
        names: vec!["Default".into(), "Red".into()],
        setvars,
        ..Default::default()
    }
}

fn setup() -> NewGameSetup {
    let mut s = NewGameSetup {
        selected_vehicle: "Mercedes Citaro".into(),
        selected_livery: "Red".into(),
        ..NewGameSetup::default()
    };
    s.custom_setvars.insert("Klima".into(), 1);
    s
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("opts-bus-config-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

mod files;
mod identity;
mod load_save;
