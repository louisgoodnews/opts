//! Unit tests of `activation`.

use super::*;
use crate::components::button::ButtonBuilder;
use crate::testing::{click, headless_app, press, release};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn counting_button(app: &mut App) -> (Entity, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let entity = {
        let mut commands = app.world_mut().commands();
        ButtonBuilder::filled("Go")
            .on_click(move |_, _| {
                c.fetch_add(1, Ordering::SeqCst);
            })
            .spawn(&mut commands)
    };
    app.update();
    (entity, count)
}

#[test]
fn release_inside_activates_once() {
    let mut app = headless_app();
    let (button, count) = counting_button(&mut app);
    click(&mut app, button);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(app.world().get::<Pressed>(button).is_none());
}

#[test]
fn press_alone_does_not_activate_and_drag_away_cancels() {
    let mut app = headless_app();
    let (button, count) = counting_button(&mut app);
    press(app.world_mut(), button);
    app.update();
    // No mouse button is held in the headless input state: the press is a cancelled one.
    assert_eq!(count.load(Ordering::SeqCst), 0, "press must not activate");
    assert!(
        app.world().get::<Pressed>(button).is_none(),
        "stale press is dropped"
    );
    // Releasing somewhere else later does nothing either.
    release(app.world_mut(), button);
    app.update();
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn release_on_a_child_counts_as_inside() {
    let mut app = headless_app();
    let (button, count) = counting_button(&mut app);
    let child = app
        .world()
        .get::<Children>(button)
        .and_then(|c| c.iter().last())
        .expect("label child");
    press(app.world_mut(), child);
    release(app.world_mut(), child);
    app.update();
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn disabled_controls_do_not_activate() {
    let mut app = headless_app();
    let (button, count) = counting_button(&mut app);
    app.world_mut().entity_mut(button).insert(Disabled);
    click(&mut app, button);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn enter_activates_the_focused_control() {
    let mut app = headless_app();
    let (button, count) = counting_button(&mut app);
    app.world_mut().entity_mut(button).insert(Focused);
    app.world_mut()
        .resource_mut::<Messages<bevy::input::keyboard::KeyboardInput>>()
        .write(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: bevy::input::keyboard::Key::Enter,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert_eq!(count.load(Ordering::SeqCst), 1);
}
