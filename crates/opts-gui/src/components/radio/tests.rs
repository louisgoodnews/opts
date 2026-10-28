//! Unit tests for `super` (`radio.rs`).

use super::*;

#[test]
fn test_radio_builder_spawn() {
    let mut app = App::new();
    let entity = {
        let mut commands = app.world_mut().commands();
        RadioBuilder::new("Option A")
            .with_group(1)
            .selected(true)
            .spawn(&mut commands)
    };

    app.update();

    let radio = app.world().get::<M3Radio>(entity);
    assert_eq!(radio.map(|r| r.is_selected), Some(true));
    assert_eq!(radio.map(|r| r.group_id), Some(Some(1)));
}

#[test]
fn test_radio_mutual_exclusion() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<RadioSelected>()
        .add_message::<Activated>()
        .add_systems(Update, radio_select_system);

    let entity_a = {
        let mut commands = app.world_mut().commands();
        RadioBuilder::new("Option A")
            .with_group(1)
            .selected(true)
            .spawn(&mut commands)
    };

    let entity_b = {
        let mut commands = app.world_mut().commands();
        RadioBuilder::new("Option B")
            .with_group(1)
            .selected(false)
            .spawn(&mut commands)
    };

    app.update();

    app.world_mut().write_message(Activated(entity_b));

    app.update();

    let radio_a = app.world().get::<M3Radio>(entity_a);
    assert_eq!(radio_a.map(|r| r.is_selected), Some(false));

    let radio_b = app.world().get::<M3Radio>(entity_b);
    assert_eq!(radio_b.map(|r| r.is_selected), Some(true));
}
