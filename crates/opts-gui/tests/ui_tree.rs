//! Stable ids, roles, labels, states and accessibility nodes of the UI tree.

use bevy::a11y::AccessibilityNode;
use bevy::prelude::*;
use opts_gui::actions::UiAction;
use opts_gui::components::{
    button::ButtonBuilder, checkbox::CheckboxBuilder, notebook::NotebookBuilder,
    switch::SwitchBuilder, text_field::TextFieldBuilder,
};
use opts_gui::interaction::state::{Disabled, DisabledReason};
use opts_gui::testing::{act, headless_app};
use opts_gui::ui_id::UiId;
use opts_gui::ui_tree::{find_entity, snapshot, UiRole};

fn fixture() -> (App, Vec<Entity>) {
    let mut app = headless_app();
    let ids = {
        let mut commands = app.world_mut().commands();
        let root = commands.spawn(Node::default()).id();
        let a = ButtonBuilder::filled("Start drive").spawn(&mut commands);
        let b = ButtonBuilder::filled("Start drive").spawn(&mut commands);
        let c = CheckboxBuilder::new("Sync")
            .checked(true)
            .spawn(&mut commands);
        let d = SwitchBuilder::new("Auto").spawn(&mut commands);
        let e = TextFieldBuilder::outlined("Search")
            .with_text("m29")
            .spawn(&mut commands);
        let f = NotebookBuilder::new()
            .with_tabs(vec!["One".into(), "Two".into()])
            .spawn(&mut commands);
        commands.entity(root).add_children(&[a, b, c, d, e, f]);
        vec![a, b, c, d, e, f]
    };
    app.update();
    app.update();
    app.update();
    (app, ids)
}

#[test]
fn every_interactive_element_gets_a_unique_path_id() {
    let (mut app, _) = fixture();
    let snap = snapshot(app.world_mut());
    let all = snap.flatten();
    let mut seen = std::collections::HashSet::new();
    for n in &all {
        assert!(seen.insert(n.id.clone()), "duplicate id {}", n.id);
    }
    assert!(all.iter().any(|n| n.id == "button:start-drive"));
    assert!(
        all.iter().any(|n| n.id == "button:start-drive-2"),
        "duplicates get a suffix"
    );
    assert!(all
        .iter()
        .any(|n| n.role == UiRole::Checkbox && n.label == "Sync"));
}

#[test]
fn ids_are_the_same_in_a_second_run() {
    let ids = |app: &mut App| -> Vec<String> {
        snapshot(app.world_mut())
            .flatten()
            .iter()
            .map(|n| n.id.clone())
            .collect()
    };
    let (mut one, _) = fixture();
    let (mut two, _) = fixture();
    assert_eq!(ids(&mut one), ids(&mut two));
}

#[test]
fn state_roles_values_and_focus_are_reported() {
    let (mut app, _) = fixture();
    let snap = snapshot(app.world_mut());
    let cb = snap
        .flatten()
        .into_iter()
        .find(|n| n.role == UiRole::Checkbox)
        .expect("checkbox");
    assert_eq!(cb.state.checked, Some("true"));
    let tf = snap
        .flatten()
        .into_iter()
        .find(|n| n.role == UiRole::Textbox)
        .expect("textbox");
    assert_eq!(tf.value.as_deref(), Some("m29"));
    assert!(tf.focusable);
    let tabs: Vec<_> = snap
        .flatten()
        .into_iter()
        .filter(|n| n.role == UiRole::Tab)
        .collect();
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs[0].state.selected, Some(true));
    assert_eq!(tabs[1].state.selected, Some(false));

    act(&mut app, UiAction::FocusNext);
    let snap = snapshot(app.world_mut());
    assert_eq!(snap.focus.as_deref(), Some("button:start-drive"));
    assert!(snap
        .find("button:start-drive")
        .is_some_and(|n| n.state.focused));
}

#[test]
fn disabled_reason_and_visibility_are_reported() {
    let (mut app, ids) = fixture();
    app.world_mut()
        .entity_mut(ids[1])
        .insert((Disabled, DisabledReason::new("Needs a saved game")));
    app.world_mut().entity_mut(ids[3]).insert(Node {
        display: Display::None,
        ..default()
    });
    app.update();
    let snap = snapshot(app.world_mut());
    let b = snap.find("button:start-drive-2").expect("button");
    assert_eq!(b.state.disabled.as_deref(), Some("Needs a saved game"));
    assert!(
        b.focusable,
        "disabled with a reason can be focused to read it"
    );
    let sw = snap
        .flatten()
        .into_iter()
        .find(|n| n.role == UiRole::Switch)
        .expect("switch");
    assert!(!sw.state.visible);
    assert!(!sw.focusable);
}

#[test]
fn accessibility_nodes_carry_role_label_and_state() {
    let (mut app, ids) = fixture();
    let node = app
        .world()
        .get::<AccessibilityNode>(ids[2])
        .expect("a11y node on the checkbox");
    assert_eq!(node.role(), accesskit::Role::CheckBox);
    assert_eq!(node.label(), Some("Sync"));
    assert_eq!(node.toggled(), Some(accesskit::Toggled::True));
    let id = app.world().get::<UiId>(ids[0]).expect("id").0.to_string();
    assert_eq!(find_entity(app.world_mut(), &id), Some(ids[0]));
}
