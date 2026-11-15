//! Unit tests of `modal`.

use super::*;

#[test]
fn modal_spawns_card_under_backdrop() {
    let mut app = App::new();
    let (root, content) = {
        let mut commands = app.world_mut().commands();
        ModalBuilder::new().spawn(&mut commands)
    };
    app.update();
    assert!(app.world().get::<ModalRoot>(root).is_some());
    assert!(app.world().get::<ModalContent>(content).is_some());
    assert_eq!(
        app.world().get::<ChildOf>(content).map(ChildOf::parent),
        Some(root)
    );
}

#[test]
fn escape_requests_cancel() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<ModalEvent>()
        .add_systems(Update, modal_escape_system);
    let root = app.world_mut().spawn(ModalRoot).id();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    let events = app.world().resource::<Messages<ModalEvent>>();
    let got: Vec<_> = events.get_cursor().read(events).copied().collect();
    assert_eq!(got, vec![ModalEvent::CancelRequested(root)]);
}
