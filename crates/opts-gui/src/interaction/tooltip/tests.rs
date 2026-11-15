//! Unit tests for `super` (`tooltip.rs`).

use super::*;
use crate::interaction::state::{DisabledReason, Focused};

#[test]
fn disabled_reason_beats_tooltip_text() {
    let t = Tooltip::new("Open");
    let r = DisabledReason::new("Needs a saved game");
    assert_eq!(
        tooltip_text(Some(&t), Some(&r), true).as_deref(),
        Some("Needs a saved game")
    );
    assert_eq!(
        tooltip_text(Some(&t), Some(&r), false).as_deref(),
        Some("Open")
    );
    assert_eq!(tooltip_text(None, None, true), None);
}

#[test]
fn keyboard_focus_shows_the_tooltip_at_once_and_blur_removes_it() {
    use crate::components::button::ButtonBuilder;
    let mut app = crate::testing::headless_app();
    let button = {
        let mut commands = app.world_mut().commands();
        ButtonBuilder::filled("Go").spawn(&mut commands)
    };
    app.update();
    app.world_mut()
        .entity_mut(button)
        .insert((Tooltip::new("Start the drive (Enter)"), Focused));
    app.update();
    app.update();
    let tips = |app: &mut App| {
        app.world_mut()
            .query::<&TooltipNode>()
            .iter(app.world())
            .count()
    };
    assert_eq!(tips(&mut app), 1);
    app.world_mut().entity_mut(button).remove::<Focused>();
    app.update();
    app.update();
    assert_eq!(tips(&mut app), 0);
}
