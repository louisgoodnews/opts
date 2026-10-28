//! Unit tests for `super` (`button.rs`).

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn test_button_on_click_and_event() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<ButtonClicked>()
        .add_message::<Activated>()
        .add_systems(Update, button_click_system);

    let clicked = Arc::new(AtomicBool::new(false));
    let clicked_clone = clicked.clone();

    let button_entity = {
        let mut commands = app.world_mut().commands();
        ButtonBuilder::filled("Test")
            .on_click(move |_commands, _entity| {
                clicked_clone.store(true, Ordering::SeqCst);
            })
            .spawn(&mut commands)
    };

    app.update();

    // Simulate an activation (pointer release inside, Confirm, `ui.click`)
    app.world_mut().write_message(Activated(button_entity));

    app.update();

    // Verify callback executed
    assert!(
        clicked.load(Ordering::SeqCst),
        "Button on_click callback should have executed"
    );

    // Verify ButtonClicked event was dispatched
    let events = app.world().resource::<Messages<ButtonClicked>>();
    let mut reader = events.get_cursor();
    let received = reader.read(events).any(|e| e.0 == button_entity);
    assert!(received, "ButtonClicked event should have been dispatched");
}
