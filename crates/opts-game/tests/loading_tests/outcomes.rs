//! Loading screen tests for retry, overlay, menu camera, CI rules and minor problems.

use super::*;

#[test]
fn retry_starts_the_load_again_and_clears_the_report() {
    let root = fresh_root("retry");
    std::fs::create_dir_all(root.join("maps")).expect("maps");
    let mut app = loading_app("Ghost Town", "MiniBus", args_for(&root));
    run(&mut app, 12);
    assert_eq!(outcome(&app), LoadOutcome::Failed);
    // The map appears; the retry finds it.
    write_map(&root, &MiniMap::simple("Ghost Town"));
    let retry = find_entity(app.world_mut(), "loading.retry").expect("retry");
    app.world_mut().write_message(Activated(retry));
    run(&mut app, 40);
    assert_ne!(
        outcome(&app),
        LoadOutcome::Failed,
        "{:?}",
        app.world().resource::<LoadReport>().problems
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_developer_overlay_is_hidden_while_the_loading_screen_is_up() {
    let root = fresh_root("overlay");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 40);
    assert_eq!(screen_count(&mut app), 1, "problems keep the screen up");
    let mut q = app.world_mut().query::<(
        &Visibility,
        &opts_game::in_game::omsi_world::DebugOverlayText,
    )>();
    let all: Vec<_> = q.iter(app.world()).map(|(v, _)| *v).collect();
    assert!(!all.is_empty(), "the overlay exists");
    assert!(all.iter().all(|v| *v == Visibility::Hidden), "{all:?}");
    let _ = std::fs::remove_dir_all(&root);
}
