//! The CI rules of `expandability.md` section 7 on a synthetic fixture (the showcase: every
//! component in every state). No OMSI assets, no window.

use bevy::prelude::*;
use opts_gui::showcase::spawn_showcase;
use opts_gui::testing::{headless_app, layout_app, rules};

fn app() -> App {
    let mut app = headless_app();
    let root = app.world_mut().spawn(Node::default()).id();
    {
        let mut commands = app.world_mut().commands();
        spawn_showcase(&mut commands, root);
    }
    for _ in 0..4 {
        app.update();
    }
    app
}

fn laid_out_app() -> App {
    let mut app = layout_app();
    let root = app
        .world_mut()
        .spawn(Node {
            width: Val::Percent(100.0),
            ..default()
        })
        .id();
    {
        let mut commands = app.world_mut().commands();
        spawn_showcase(&mut commands, root);
    }
    for _ in 0..6 {
        app.update();
    }
    app
}

#[test]
fn laid_out_elements_have_a_real_size() {
    let mut app = laid_out_app();
    let snap = opts_gui::ui_tree::snapshot(app.world_mut());
    let with_rect = snap.flatten().iter().filter(|n| n.rect.is_some()).count();
    assert!(with_rect > 40, "layout ran for {with_rect} nodes");
}

#[test]
fn no_interactive_target_is_smaller_than_32_by_32_in_the_real_layout() {
    let mut app = laid_out_app();
    let bad = rules::small_targets(app.world_mut());
    assert!(bad.is_empty(), "targets below 32x32:\n{}", bad.join("\n"));
}

#[test]
fn no_interactive_target_is_smaller_than_32_by_32() {
    let mut app = app();
    let bad = rules::small_targets(app.world_mut());
    assert!(bad.is_empty(), "targets below 32x32:\n{}", bad.join("\n"));
}

#[test]
fn no_text_is_smaller_than_12_px() {
    let mut app = app();
    let bad = rules::small_text(app.world_mut());
    assert!(bad.is_empty(), "text below 12 px:\n{}", bad.join("\n"));
}

#[test]
fn every_disabled_control_has_a_reason_or_the_not_yet_marker() {
    let mut app = app();
    let bad = rules::disabled_without_reason(app.world_mut());
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn keyboard_alone_reaches_and_activates_every_interactive_element() {
    let mut app = app();
    let report = rules::keyboard_traversal(&mut app);
    assert!(
        report.missed.is_empty(),
        "never focused: {:?}",
        report.missed
    );
    assert!(
        report.reached.len() > 40,
        "the showcase has many controls, reached {}",
        report.reached.len()
    );
    assert!(
        report.inert.is_empty(),
        "focused but Confirm did nothing: {:?}",
        report.inert
    );
    assert!(
        report.activated.len() > 30,
        "activated {}",
        report.activated.len()
    );
}

#[test]
fn no_hard_coded_colours_outside_the_token_files() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let bad = rules::hardcoded_colours(&src);
    assert!(bad.is_empty(), "hard-coded colours:\n{}", bad.join("\n"));
}
