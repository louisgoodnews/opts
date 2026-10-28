//! Unit tests for `super` (`switch.rs`).

use super::*;

#[test]
fn test_switch_builder_spawn() {
    let mut app = App::new();
    let entity = {
        let mut commands = app.world_mut().commands();
        SwitchBuilder::new("Automatic Routing")
            .with_value(true)
            .spawn(&mut commands)
    };

    app.update();

    let switch = app.world().get::<M3Switch>(entity);
    assert_eq!(switch.map(|s| s.is_on), Some(true));
}

#[test]
fn test_switch_toggle_system() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<SwitchToggled>()
        .add_message::<Activated>()
        .add_systems(Update, switch_toggle_system);

    let entity = {
        let mut commands = app.world_mut().commands();
        SwitchBuilder::standalone().spawn(&mut commands)
    };

    app.update();

    app.world_mut().write_message(Activated(entity));

    app.update();

    let switch = app.world().get::<M3Switch>(entity);
    assert_eq!(switch.map(|s| s.is_on), Some(true));

    let events = app.world().resource::<Messages<SwitchToggled>>();
    let mut reader = events.get_cursor();
    let received = reader.read(events).find(|e| e.entity == entity);
    assert_eq!(received.map(|e| e.is_on), Some(true));
}
