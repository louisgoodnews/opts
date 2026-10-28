//! Unit tests for `super` (`checkbox.rs`).

use super::*;

#[test]
fn test_checkbox_builder_spawn() {
    let mut app = App::new();
    let entity = {
        let mut commands = app.world_mut().commands();
        CheckboxBuilder::new("Test option")
            .checked(true)
            .spawn(&mut commands)
    };

    app.update();

    let checkbox = app.world().get::<M3Checkbox>(entity);
    assert_eq!(checkbox.map(|c| c.state), Some(CheckState::Checked));
}

#[test]
fn test_checkbox_toggle_system() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<CheckboxToggled>()
        .add_message::<Activated>()
        .add_systems(Update, checkbox_toggle_system);

    let entity = {
        let mut commands = app.world_mut().commands();
        CheckboxBuilder::standalone().spawn(&mut commands)
    };

    app.update();

    app.world_mut().write_message(Activated(entity));

    app.update();

    let checkbox = app.world().get::<M3Checkbox>(entity);
    assert_eq!(checkbox.map(|c| c.state), Some(CheckState::Checked));

    let events = app.world().resource::<Messages<CheckboxToggled>>();
    let mut reader = events.get_cursor();
    let received = reader.read(events).find(|e| e.entity == entity);
    assert_eq!(received.map(|e| e.state), Some(CheckState::Checked));
}
