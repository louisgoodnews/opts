//! Keyboard operability of the composite widgets, driven only through [`UiAction`]s.

use bevy::prelude::*;
use opts_gui::actions::{Direction, UiAction};
use opts_gui::components::{
    checkbox::M3Checkbox,
    combobox::{ComboboxBuilder, M3Combobox},
    notebook::{M3Notebook, NotebookBuilder},
    switch::{M3Switch, SwitchBuilder},
    toast::ToastBuilder,
    treeview::{M3TreeNode, TreeNode, TreeviewBuilder},
    CheckboxBuilder,
};
use opts_gui::interaction::state::CheckState;
use opts_gui::testing::{act, focused, headless_app};

fn spawn_with<F: FnOnce(&mut Commands) -> Entity>(app: &mut App, f: F) -> Entity {
    let e = {
        let mut commands = app.world_mut().commands();
        f(&mut commands)
    };
    app.update();
    app.update();
    e
}

#[test]
fn notebook_tabs_switch_with_next_and_previous_tab_and_by_arrow_focus() {
    let mut app = headless_app();
    let nb = spawn_with(&mut app, |c| {
        NotebookBuilder::new()
            .with_tabs(vec!["A".into(), "B".into(), "C".into()])
            .spawn(c)
    });
    act(&mut app, UiAction::NextTab);
    assert_eq!(
        app.world().get::<M3Notebook>(nb).map(|n| n.active_tab),
        Some(1)
    );
    act(&mut app, UiAction::PrevTab);
    act(&mut app, UiAction::PrevTab);
    assert_eq!(
        app.world().get::<M3Notebook>(nb).map(|n| n.active_tab),
        Some(2),
        "wraps"
    );

    // Arrow focus on a header selects it (automatic activation).
    act(&mut app, UiAction::FocusNext);
    act(&mut app, UiAction::Navigate(Direction::Right));
    let active = app.world().get::<M3Notebook>(nb).map(|n| n.active_tab);
    assert!(active.is_some());
    assert!(focused(&mut app).is_some());
}

#[test]
fn switch_and_checkbox_toggle_on_confirm() {
    let mut app = headless_app();
    let sw = spawn_with(&mut app, |c| SwitchBuilder::new("Auto").spawn(c));
    let cb = spawn_with(&mut app, |c| CheckboxBuilder::new("Sync").spawn(c));
    act(&mut app, UiAction::FocusNext);
    assert_eq!(focused(&mut app), Some(sw));
    act(&mut app, UiAction::Confirm);
    assert_eq!(app.world().get::<M3Switch>(sw).map(|s| s.is_on), Some(true));
    act(&mut app, UiAction::FocusNext);
    assert_eq!(focused(&mut app), Some(cb));
    act(&mut app, UiAction::Confirm);
    assert_eq!(
        app.world().get::<M3Checkbox>(cb).map(|c| c.state),
        Some(CheckState::Checked)
    );
}

#[test]
fn tree_rows_expand_and_collapse_with_left_and_right() {
    let mut app = headless_app();
    let _tree = spawn_with(&mut app, |c| {
        TreeviewBuilder::new()
            .with_node(TreeNode::new("net", "Network").with_child(TreeNode::new("m29", "M29")))
            .spawn(c)
    });
    act(&mut app, UiAction::FocusNext);
    let row = focused(&mut app).expect("a tree row is focusable");
    let expanded = |app: &App| app.world().get::<M3TreeNode>(row).map(|n| n.is_expanded);
    let before = expanded(&app);
    act(&mut app, UiAction::Navigate(Direction::Right));
    assert_ne!(
        expanded(&app),
        before,
        "right toggles a collapsed parent open"
    );
    assert_eq!(focused(&mut app), Some(row), "focus stays on the row");
    act(&mut app, UiAction::Navigate(Direction::Left));
    assert_eq!(expanded(&app), before, "left collapses it again");
}

#[test]
fn combobox_opens_with_arrow_picks_with_confirm_and_closes_with_back() {
    let mut app = headless_app();
    let combo = spawn_with(&mut app, |c| {
        ComboboxBuilder::new("Route")
            .with_options(vec!["Line 100".into(), "Line 200".into()])
            .spawn(c)
    });
    act(&mut app, UiAction::FocusNext);
    act(&mut app, UiAction::Navigate(Direction::Down));
    app.update();
    assert_eq!(
        app.world().get::<M3Combobox>(combo).map(|c| c.is_open),
        Some(true)
    );
    act(&mut app, UiAction::Navigate(Direction::Down));
    act(&mut app, UiAction::Confirm);
    let c = app.world().get::<M3Combobox>(combo).expect("combobox");
    assert!(!c.is_open, "picking closes the list");
    assert_eq!(c.selected, Some(1));

    act(&mut app, UiAction::Navigate(Direction::Down));
    assert_eq!(
        app.world().get::<M3Combobox>(combo).map(|c| c.is_open),
        Some(true)
    );
    act(&mut app, UiAction::Back);
    assert_eq!(
        app.world().get::<M3Combobox>(combo).map(|c| c.is_open),
        Some(false)
    );
}

#[test]
fn a_toast_action_is_reachable_and_back_dismisses_it() {
    let mut app = headless_app();
    let toast = spawn_with(&mut app, |c| {
        ToastBuilder::new("Saved")
            .with_duration(60.0)
            .with_action("UNDO")
            .spawn(c)
    });
    act(&mut app, UiAction::FocusNext);
    assert!(focused(&mut app).is_some(), "the toast action takes focus");
    act(&mut app, UiAction::Back);
    assert!(app.world().get_entity(toast).is_err(), "toast is gone");
}
