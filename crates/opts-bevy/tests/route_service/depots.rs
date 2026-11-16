//! Choice of the depot among several `.hof` files of one map folder.

use super::*;

#[test]
fn of_several_depots_named_after_the_map_the_one_with_its_lines_is_chosen() {
    let dir = std::env::temp_dir().join(format!("opts-depot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    // `Berlin.hof` comes first by name and has destinations but no routes.
    let no_routes = HOF.split("[infosystem_trip]").next().expect("head");
    std::fs::write(dir.join("Berlin.hof"), no_routes.replace("Depot", "Berlin")).expect("write");
    std::fs::write(dir.join("Spandau.hof"), HOF.replace("Depot", "Spandau")).expect("write");
    let depot = opts_bevy::route::find_depot(&dir, &dir, &["Berlin-Spandau"]).expect("depot");
    assert_eq!(depot.name, "Spandau");
    assert_eq!(depot.routes.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_depot_sharing_the_map_folders_number_is_chosen_when_no_name_matches() {
    let dir = std::env::temp_dir().join(format!("opts-depot-tokens-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    // `000_S-Bahn-Berlin.hof` comes first by name; `Stadtrundfahrt Hamburg.hof` shares a word only;
    // `1_HH109-2_HHA.hof` belongs to the second Hamburg map (`Hamburg109_2`).
    for (file, name) in [
        ("000_S-Bahn-Berlin.hof", "S-Bahn"),
        ("1_HH109-2_HHA.hof", "Hamburg 2 (HHA)"),
        ("Stadtrundfahrt Hamburg.hof", "Rundfahrt"),
        ("1_HH109_HHA.hof", "Hamburg (HHA)"),
    ] {
        std::fs::write(dir.join(file), HOF.replace("Depot", name)).expect("write");
    }
    let names = ["Hamburg Tag & Nacht", "Hamburg109"];
    let depot = opts_bevy::route::find_depot(&dir, &dir, &names).expect("depot");
    assert_eq!(depot.name, "Hamburg (HHA)");
    let second = ["Hamburg Tag & Nacht", "Hamburg109_2"];
    let depot = opts_bevy::route::find_depot(&dir, &dir, &second).expect("depot");
    assert_eq!(depot.name, "Hamburg 2 (HHA)");
    let _ = std::fs::remove_dir_all(&dir);
}
