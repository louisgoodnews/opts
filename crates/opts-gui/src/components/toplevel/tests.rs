//! Unit tests for `super` (`toplevel.rs`).

use super::*;

#[test]
fn test_toplevel_builder_spawn() {
    let mut app = App::new();
    let entity = {
        let mut commands = app.world_mut().commands();
        ToplevelBuilder::new("Settings Dialog")
            .modal(true)
            .spawn(&mut commands)
    };

    app.update();

    let toplevel = app.world().get::<M3Toplevel>(entity);
    assert_eq!(toplevel.map(|t| t.title.as_str()), Some("Settings Dialog"));
    assert_eq!(toplevel.map(|t| t.is_modal), Some(true));
}
