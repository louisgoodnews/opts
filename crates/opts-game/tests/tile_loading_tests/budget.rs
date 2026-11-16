//! Integration tests for the resumable, per-frame tile build under a small budget.
use super::*;

// --- M-0089: the tile build is resumable and bounded per frame ---------------------------------

fn tile_roots(app: &mut App, key: (i32, i32)) -> usize {
    app.world_mut()
        .query::<&opts_game::in_game::omsi_world::TileRoot>()
        .iter(app.world())
        .filter(|r| r.0 == key)
        .count()
}

#[test]
fn a_tile_is_built_over_several_frames_under_a_small_budget_and_ends_complete() {
    let (mut app, root) = drive("tl-budget", &[("gameplay.tile_radius", "1")]);
    assert_eq!(resident(&app), vec![(0, 0), (1, 0)]);
    // A budget that lets one step run per frame (the first step of a frame is always allowed).
    set_budget(&mut app, std::time::Duration::from_nanos(1));
    move_camera_to(&mut app, (4, 0));
    let mut frames_building = 0;
    let mut seen_resident_while_building = false;
    for _ in 0..80 {
        app.update();
        if let Some(key) = building(&app) {
            frames_building += 1;
            seen_resident_while_building |= resident(&app).contains(&key);
        }
    }
    assert!(
        frames_building >= 6,
        "a tile took only {frames_building} frames"
    );
    assert!(
        seen_resident_while_building,
        "a tile in progress is resident with its root"
    );
    assert_eq!(building(&app), None, "everything is built");
    assert_eq!(resident(&app), vec![(3, 0), (4, 0)]);
    assert_eq!(tile_roots(&mut app, (3, 0)), 1);
    assert_eq!(tile_roots(&mut app, (4, 0)), 1);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_small_budget_builds_the_same_tiles_as_one_tile_per_frame() {
    let go = |tag: &str, budget: std::time::Duration| {
        let (mut app, root) = drive(tag, &[("gameplay.tile_radius", "2")]);
        set_budget(&mut app, budget);
        move_camera_to(&mut app, (3, 0));
        run(&mut app, 200);
        let tiles = resident(&app);
        let _ = std::fs::remove_dir_all(&root);
        tiles
    };
    let old = go("tl-same-old", std::time::Duration::ZERO);
    let new = go("tl-same-new", std::time::Duration::from_nanos(1));
    assert_eq!(old, new);
    // Radius 2 around tile 3 wants 1 to 4 (5 does not exist); tile 0 is 3 away, inside the
    // hysteresis of 1, so it stays.
    assert_eq!(new, vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]);
}

#[test]
fn a_tile_that_is_unloaded_while_it_builds_leaves_nothing_behind() {
    let (mut app, root) = drive("tl-cancel", &[("gameplay.tile_radius", "1")]);
    set_budget(&mut app, std::time::Duration::from_nanos(1));
    move_camera_to(&mut app, (4, 0));
    // Run until a tile is in progress, then take the camera back before it is finished.
    let mut key = None;
    for _ in 0..30 {
        app.update();
        key = building(&app);
        if key.is_some() {
            break;
        }
    }
    let key = key.expect("a tile is being built");
    assert_eq!(tile_roots(&mut app, key), 1);
    move_camera_to(&mut app, (0, 0));
    run(&mut app, 80);
    assert_eq!(building(&app), None);
    assert_eq!(
        tile_roots(&mut app, key),
        0,
        "the half-built tile was despawned"
    );
    assert_eq!(resident(&app), vec![(0, 0), (1, 0)]);
    let _ = std::fs::remove_dir_all(&root);
}
