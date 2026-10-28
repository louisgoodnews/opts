//! Unit tests for `super` (`configs_page.rs`).

use super::*;
use crate::new_game_screen::repaints::VehicleRepaints;

fn cat() -> GameCatalogue {
    GameCatalogue {
        maps: Vec::new(),
        vehicles: vec![VehicleItem::named("c2", "Solo", "C2")],
    }
}

fn repaints(_: &VehicleItem) -> VehicleRepaints {
    VehicleRepaints {
        default_name: "Default".into(),
        names: vec!["Default".into(), "Red".into()],
        ..Default::default()
    }
}

#[test]
fn rows_show_favourites_first_then_recent_and_block_what_cannot_spawn() {
    let mut store = BusConfigurations::default();
    let mut a = BusConfiguration::new("Old", "c2", "", vec![], 1);
    a.last_used = 10;
    store.save(a);
    let mut b = BusConfiguration::new("Recent", "c2", "Red", vec![], 1);
    b.last_used = 99;
    store.save(b);
    let (fav, _) = store.save(BusConfiguration::new("Fav", "c2", "Gone", vec![], 1));
    store.set_favourite(&fav, true);
    store.save(BusConfiguration::new("Lost", "missing", "", vec![], 1));
    let rows = rows_of(&store, &cat(), repaints);
    let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["Fav", "Recent", "Old", "Lost"]);
    // A repaint that is gone still spawns (the default is used) and says so.
    assert!(rows[0].blocked.is_empty());
    assert!(!rows[0].notice.is_empty());
    assert!(rows[1].detail.contains("Red"));
    assert!(!rows[3].blocked.is_empty());
}

#[test]
fn the_active_vehicle_gets_a_free_name() {
    let mut store = BusConfigurations::default();
    assert_eq!(free_name(&store, "Solo C2"), "Solo C2");
    store.save(BusConfiguration::new("Solo C2", "c2", "", vec![], 1));
    assert_eq!(free_name(&store, "Solo C2"), "Solo C2 (2)");
    store.save(BusConfiguration::new("Solo C2 (2)", "c2", "", vec![], 1));
    assert_eq!(free_name(&store, "solo c2"), "solo c2 (3)");
}

fn library() -> ConfigLibrary {
    let mut lib = ConfigLibrary::at(None);
    lib.store
        .save(BusConfiguration::new("Morning", "c2", "", vec![], 1));
    lib.store
        .save(BusConfiguration::new("Evening", "c2", "", vec![], 1));
    lib
}

#[test]
fn a_rename_takes_the_typed_name_and_refuses_empty_or_taken_ones() {
    let mut lib = library();
    let mut menu = VehicleMenu {
        configs: load_view(&lib, &cat()),
        ..Default::default()
    };
    start_rename(&mut menu, "morning");
    assert_eq!(
        menu.configs.renaming,
        Some(("morning".into(), "Morning".into()))
    );

    // Taken (ignoring case): refused, the field stays open.
    menu.configs.renaming = Some(("morning".into(), "evening".into()));
    commit_rename(&mut menu, &mut lib, &cat());
    assert!(menu.status.contains("already used"));
    assert!(menu.configs.renaming.is_some());
    // Empty: refused as well.
    menu.configs.renaming = Some(("morning".into(), "  ".into()));
    commit_rename(&mut menu, &mut lib, &cat());
    assert!(menu.status.contains("already used"));

    menu.configs.renaming = Some(("morning".into(), " Early shift ".into()));
    commit_rename(&mut menu, &mut lib, &cat());
    assert_eq!(menu.status, "Renamed to Early shift.");
    assert!(menu.configs.renaming.is_none());
    assert!(menu.configs.rows.iter().any(|r| r.name == "Early shift"));
    assert_eq!(
        lib.store.find("morning").map(|c| c.name.as_str()),
        Some("Early shift")
    );
}
