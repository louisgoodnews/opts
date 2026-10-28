//! Unit tests for `super` (`focus.rs`).

use super::*;
use crate::actions::Direction;
use crate::components::{button::ButtonBuilder, modal::ModalBuilder};
use crate::interaction::state::{Disabled, DisabledReason};
use crate::testing::{act, button_column, focused, headless_app};

#[test]
fn tab_walks_the_layout_order_and_wraps() {
    let mut app = headless_app();
    let b = button_column(&mut app, 3);
    act(&mut app, UiAction::NextRegion);
    assert_eq!(focused(&mut app), Some(b[0]));
    act(&mut app, UiAction::NextRegion);
    act(&mut app, UiAction::NextRegion);
    assert_eq!(focused(&mut app), Some(b[2]));
    act(&mut app, UiAction::NextRegion);
    assert_eq!(focused(&mut app), Some(b[0]), "wraps");
    act(&mut app, UiAction::PrevRegion);
    assert_eq!(focused(&mut app), Some(b[2]));
}

#[test]
fn hidden_and_disabled_controls_never_take_focus() {
    let mut app = headless_app();
    let b = button_column(&mut app, 4);
    app.world_mut().entity_mut(b[1]).insert(Disabled);
    app.world_mut().entity_mut(b[2]).insert(Node {
        display: Display::None,
        ..default()
    });
    app.update();
    let mut seen = Vec::new();
    for _ in 0..3 {
        act(&mut app, UiAction::FocusNext);
        seen.push(focused(&mut app));
    }
    assert_eq!(seen, vec![Some(b[0]), Some(b[3]), Some(b[0])]);
}

#[test]
fn disabled_with_a_reason_can_be_focused_to_read_it() {
    let mut app = headless_app();
    let b = button_column(&mut app, 2);
    app.world_mut()
        .entity_mut(b[1])
        .insert((Disabled, DisabledReason::new("Needs a saved game")));
    app.update();
    act(&mut app, UiAction::FocusNext);
    act(&mut app, UiAction::FocusNext);
    assert_eq!(focused(&mut app), Some(b[1]));
}

#[test]
fn arrows_follow_layout_order_without_geometry() {
    let mut app = headless_app();
    let b = button_column(&mut app, 3);
    act(&mut app, UiAction::Navigate(Direction::Down));
    assert_eq!(focused(&mut app), Some(b[0]));
    act(&mut app, UiAction::Navigate(Direction::Down));
    assert_eq!(focused(&mut app), Some(b[1]));
    act(&mut app, UiAction::Navigate(Direction::Up));
    assert_eq!(focused(&mut app), Some(b[0]));
}

#[test]
fn modal_traps_focus_and_gives_it_back() {
    let mut app = headless_app();
    let b = button_column(&mut app, 2);
    act(&mut app, UiAction::FocusNext);
    assert_eq!(focused(&mut app), Some(b[0]));
    let (root, content) = {
        let mut commands = app.world_mut().commands();
        ModalBuilder::new().spawn(&mut commands)
    };
    let inner = {
        let mut commands = app.world_mut().commands();
        let inner = ButtonBuilder::filled("OK").spawn(&mut commands);
        commands.entity(content).add_child(inner);
        inner
    };
    app.update();
    app.update();
    assert_eq!(focused(&mut app), Some(inner), "focus moves into the modal");
    act(&mut app, UiAction::FocusNext);
    assert_eq!(focused(&mut app), Some(inner), "trapped");
    app.world_mut().entity_mut(root).despawn();
    app.update();
    app.update();
    assert_eq!(focused(&mut app), Some(b[0]), "restored");
}

#[test]
fn regions_are_jumped_by_tab() {
    let mut app = headless_app();
    let (a, b) = {
        let mut commands = app.world_mut().commands();
        let rail = commands.spawn(FocusRegion(Landmark::Rail)).id();
        let content = commands.spawn(FocusRegion(Landmark::Content)).id();
        let a1 = ButtonBuilder::filled("a1").spawn(&mut commands);
        let a2 = ButtonBuilder::filled("a2").spawn(&mut commands);
        let c1 = ButtonBuilder::filled("c1").spawn(&mut commands);
        commands.entity(rail).add_children(&[a1, a2]);
        commands.entity(content).add_child(c1);
        (a1, c1)
    };
    app.update();
    act(&mut app, UiAction::NextRegion);
    assert_eq!(focused(&mut app), Some(a));
    act(&mut app, UiAction::NextRegion);
    assert_eq!(focused(&mut app), Some(b));
}

#[test]
fn marker_and_input_focus_agree() {
    let mut app = headless_app();
    let b = button_column(&mut app, 2);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(b[1], FocusCause::Navigated);
    app.update();
    assert_eq!(
        focused(&mut app),
        Some(b[1]),
        "InputFocus wins when it changes"
    );
    app.world_mut().entity_mut(b[1]).remove::<Focused>();
    app.world_mut().entity_mut(b[0]).insert(Focused);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(b[0]));
    assert_eq!(focused(&mut app), Some(b[0]));
}

#[test]
fn default_focus_is_taken_when_nothing_is_focused() {
    let mut app = headless_app();
    let b = button_column(&mut app, 2);
    app.world_mut().entity_mut(b[1]).insert(DefaultFocus);
    app.update();
    app.update();
    assert_eq!(focused(&mut app), Some(b[1]));
}

#[test]
fn a_focused_control_that_gets_hidden_loses_focus() {
    let mut app = headless_app();
    let b = button_column(&mut app, 2);
    act(&mut app, UiAction::FocusNext);
    app.world_mut().entity_mut(b[0]).insert(Node {
        display: Display::None,
        ..default()
    });
    app.update();
    assert_eq!(focused(&mut app), None);
}
