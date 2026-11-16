//! Integration tests for tile loading modes: load all, radius, automatic and changed settings.
use super::*;

#[test]
fn load_all_has_every_tile_resident_when_the_drive_starts_and_keeps_them() {
    let (mut app, root) = drive("tl-all", &[("gameplay.load_all_tiles", "true")]);
    assert_eq!(mode(&app), TileMode::All);
    assert_eq!(resident(&app), TILES.to_vec(), "all five after loading");
    move_camera_to(&mut app, (4, 0));
    run(&mut app, 30);
    assert_eq!(resident(&app), TILES.to_vec(), "none is unloaded far away");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_radius_keeps_only_the_tiles_around_the_centre_tile() {
    let (mut app, root) = drive("tl-radius", &[("gameplay.tile_radius", "1")]);
    assert_eq!(mode(&app), TileMode::Radius(1));
    assert_eq!(resident(&app), vec![(0, 0), (1, 0)], "3 x 3 around (0, 0)");
    // Far along the row: the tiles near the start leave (beyond radius + 1), those within the
    // radius of the new centre tile come in.
    move_camera_to(&mut app, (4, 0));
    run(&mut app, 30);
    assert_eq!(resident(&app), vec![(3, 0), (4, 0)]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_larger_radius_loads_more_tiles_while_loading() {
    let (app, root) = drive("tl-larger", &[("gameplay.tile_radius", "3")]);
    assert_eq!(mode(&app), TileMode::Radius(3));
    assert_eq!(resident(&app), vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn automatic_without_a_hardware_profile_is_the_old_3x3() {
    let (app, root) = drive("tl-auto", &[]);
    assert_eq!(mode(&app), TileMode::Radius(1));
    assert_eq!(resident(&app), vec![(0, 0), (1, 0)]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_changed_setting_is_followed_during_the_drive() {
    let (mut app, root) = drive("tl-change", &[("gameplay.tile_radius", "1")]);
    assert_eq!(resident(&app), vec![(0, 0), (1, 0)]);
    // Switching *load all* on loads the rest, one tile per frame.
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("gameplay.load_all_tiles", "true");
    run(&mut app, 30);
    assert_eq!(mode(&app), TileMode::All);
    assert_eq!(resident(&app), TILES.to_vec());
    // And off again: the camera is at the start, so the tiles beyond radius + 1 leave.
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("gameplay.load_all_tiles", "false");
    run(&mut app, 30);
    assert_eq!(mode(&app), TileMode::Radius(1));
    assert_eq!(resident(&app), vec![(0, 0), (1, 0), (2, 0)]);
    let _ = std::fs::remove_dir_all(&root);
}
