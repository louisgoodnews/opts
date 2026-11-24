//! Keyboard operability of the combobox, driven only through [`UiAction`]s: opening, stepping,
//! picking and closing, and that the pick does not depend on the order of the systems.

use bevy::prelude::*;
use opts_gui::actions::{Direction, UiAction};
use opts_gui::components::combobox::{ComboboxBuilder, M3Combobox};
use opts_gui::testing::{act, headless_app};

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
fn combobox_opens_with_arrow_picks_with_confirm_and_closes_with_back() {
    let mut app = headless_app();
    let combo = spawn_route(&mut app);
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

fn spawn_route(app: &mut App) -> Entity {
    spawn_with(app, |c| {
        ComboboxBuilder::new("Route")
            .with_options(vec!["Line 100".into(), "Line 200".into()])
            .spawn(c)
    })
}

/// The combobox steps of [`combobox_opens_with_arrow_picks_with_confirm_and_closes_with_back`] in
/// a fresh app: open with Down, step to the second line, pick it with Confirm.
fn open_step_and_pick() -> Option<usize> {
    let mut app = headless_app();
    let combo = spawn_route(&mut app);
    act(&mut app, UiAction::FocusNext);
    act(&mut app, UiAction::Navigate(Direction::Down));
    act(&mut app, UiAction::Navigate(Direction::Down));
    act(&mut app, UiAction::Confirm);
    app.world()
        .get::<M3Combobox>(combo)
        .and_then(|c| c.selected)
}

/// The systems run in a different order from one run to the next (the multi-threaded executor),
/// and the picked line used to depend on it: a focus event written when the list opened landed
/// after the Down press that opened it, so that press moved focus a second time. Every fresh app
/// must pick line 2.
#[test]
fn combobox_pick_after_opening_does_not_depend_on_system_order() {
    for run in 0..300 {
        assert_eq!(open_step_and_pick(), Some(1), "run {run}");
    }
}
