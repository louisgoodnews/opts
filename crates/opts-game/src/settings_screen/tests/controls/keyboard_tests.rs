//! Keyboard use: every control of every category can be focused and confirmed.

use super::*;

/// Keyboard: focusing any control of a category and pressing Confirm uses it (the page's own
/// traversal; the library helper would also press the category buttons, which changes the page).
#[test]
fn every_control_of_every_category_can_be_used_with_the_keyboard() {
    use opts_gui::actions::UiAction;
    use opts_gui::interaction::activation::Activated;
    use opts_gui::interaction::focus::FocusEvent;
    for cat in crate::settings_registry::Category::ALL {
        let (mut app, _, _) = page_app(&format!("kbd-{}", cat.id()), cat.id());
        let prefix = format!("settings/{}/", cat.id());
        let ids: Vec<(Entity, String)> = app
            .world_mut()
            .query::<(Entity, &UiId, &opts_gui::interaction::focus::Focusable)>()
            .iter(app.world())
            .filter(|(e, u, _)| {
                u.0.starts_with(&prefix) && app.world().get::<Disabled>(*e).is_none()
            })
            .map(|(e, u, _)| (e, u.0.to_string()))
            .collect();
        assert!(!ids.is_empty(), "{}: no controls", cat.id());
        for (_, id) in ids {
            // A language change rebuilds the page: look the control up again by its id.
            let e = by_id(&mut app, &id).expect("control exists");
            if app.world().get::<Disabled>(e).is_some() {
                // An earlier control locked it (the lighting mode switch greys out the
                // governor's target frame rate).
                continue;
            }
            app.world_mut().write_message(FocusEvent::Set(e));
            app.update();
            app.update();
            let before = opts_gui::testing::focused(&mut app);
            let mut seen = false;
            app.world_mut().write_message(UiAction::Confirm);
            for _ in 0..2 {
                app.update();
                let msgs = app.world().resource::<Messages<Activated>>();
                let mut cur = msgs.get_cursor();
                seen |= cur.read(msgs).any(|a| a.0 == e);
            }
            let f = opts_gui::testing::focused(&mut app)
                .and_then(|f| app.world().get::<UiId>(f).map(|u| u.0.to_string()));
            assert!(seen, "{id}: Confirm did not activate it (focus before confirm {before:?}, now on {f:?}, target {e:?}, locale {})", app.world().resource::<opts_gui::l10n::Localization>().locale());
            // Rebuilds replace entities; settle before the next control.
            for _ in 0..10 {
                app.update();
            }
        }
    }
}
