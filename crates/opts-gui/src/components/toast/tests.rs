//! Unit tests for `super` (`toast.rs`).

use super::*;

#[test]
fn test_toast_builder_spawn() {
    let mut app = App::new();
    let entity = {
        let mut commands = app.world_mut().commands();
        ToastBuilder::new("Notice text")
            .with_action("CLOSE")
            .spawn(&mut commands)
    };

    app.update();

    let toast = app.world().get::<M3Toast>(entity);
    assert_eq!(toast.map(|t| t.message.as_str()), Some("Notice text"));
}
