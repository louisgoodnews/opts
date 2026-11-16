//! Rules every step obeys: CI rules, keyboard, layout and sources.

use super::*;

#[test]
fn every_step_meets_the_ci_rules() {
    let mut app = wizard("rules");
    for id in [
        "wizard.stepper.where",
        "wizard.stepper.what",
        "wizard.stepper.when",
        "wizard.stepper.review",
    ] {
        press(&mut app, id);
        for _ in 0..4 {
            app.update();
        }
        let small = rules::small_targets(app.world_mut());
        assert!(small.is_empty(), "{id}: {}", small.join("\n"));
        let text = rules::small_text(app.world_mut());
        assert!(text.is_empty(), "{id}: {}", text.join("\n"));
        let dis = rules::disabled_without_reason(app.world_mut());
        assert!(dis.is_empty(), "{id}: {}", dis.join("\n"));
    }
    cleanup("rules");
}

#[test]
fn the_tree_names_the_controls_of_each_step() {
    let mut app = wizard("tree");
    let snap = snapshot(app.world_mut());
    for id in [
        "wizard.where.search",
        "wizard.where.list",
        "wizard.footer.next",
        "wizard.footer.back",
        "wizard.stepper.review",
    ] {
        assert!(snap.find(id).is_some(), "{id} missing");
    }
    let _ = FooterButton::Drive;
    cleanup("tree");
}

#[test]
fn every_step_is_fully_keyboard_operable() {
    let mut app = wizard("keys");
    for id in [
        "wizard.stepper.where",
        "wizard.stepper.what",
        "wizard.stepper.when",
        "wizard.stepper.review",
    ] {
        press(&mut app, id);
        let report = rules::keyboard_traversal(&mut app);
        assert!(
            report.missed.is_empty(),
            "{id}: never focused {:?}",
            report.missed
        );
        // Confirm on a control that changes the step can hide later controls of the walk
        // (the review's Drive); every other control must react.
        let inert: Vec<_> = report
            .inert
            .iter()
            .filter(|i| !i.contains("footer.drive"))
            .collect();
        assert!(inert.is_empty(), "{id}: Confirm did nothing on {inert:?}");
        // The step jumps (stepper buttons) change the screen; return to the step under test.
        press(&mut app, id);
    }
    cleanup("keys");
}

mod layout;
mod sources;
