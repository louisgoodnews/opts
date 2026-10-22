//! Every control usable; key rebinding; one descriptor end to end.

use super::content::outcome_text;
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

/// Controls: *Change key* waits for the next key press and stores it; a key that another action
/// has is refused with its reason on the page, and Esc cancels.
#[test]
fn a_vehicle_key_is_rebound_by_pressing_it() {
    let (mut app, _, db) = page_app("rebind-keys", "controls");
    let change = "settings/controls/controls.vehicle.lights/change";
    let press = |app: &mut App, key: KeyCode| {
        use bevy::input::keyboard::{Key, KeyboardInput};
        for state in [
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
        ] {
            app.world_mut().write_message(KeyboardInput {
                key_code: key,
                logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
            app.update();
        }
    };
    let b = by_id(&mut app, change).expect("change button");
    click(&mut app, b);
    app.update();
    assert_eq!(
        app.world().resource::<SettingsPageState>().rebinding,
        Some("controls.vehicle.lights")
    );
    press(&mut app, KeyCode::KeyJ);
    app.update();
    assert_eq!(text(&app, "controls.vehicle.lights"), "J");
    assert_eq!(stored(&db, "controls.vehicle.lights").as_deref(), Some("J"));
    assert!(app
        .world()
        .resource::<SettingsPageState>()
        .rebinding
        .is_none());
    // The engine start key is taken: refused, the old key stays, the reason is on the page.
    let b = by_id(&mut app, change).expect("change button");
    click(&mut app, b);
    app.update();
    press(&mut app, KeyCode::KeyM);
    app.update();
    assert_eq!(text(&app, "controls.vehicle.lights"), "J");
    let notice = app
        .world()
        .resource::<SettingsPageState>()
        .notice
        .clone()
        .expect("a reason");
    assert!(notice.contains("engine_start"), "{notice}");
    // Esc cancels without a change.
    let b = by_id(&mut app, change).expect("change button");
    click(&mut app, b);
    app.update();
    press(&mut app, KeyCode::Escape);
    app.update();
    assert_eq!(text(&app, "controls.vehicle.lights"), "J");
    assert!(app.world().resource::<SettingsPageState>().notice.is_none());
}

/// The acceptance criterion of M-0063: one descriptor, and the setting is in the UI, in the saved
/// file, in the control protocol and in the generated reference page, with no other edit.
#[test]
fn one_descriptor_reaches_ui_persistence_protocol_and_reference() {
    use crate::settings_registry::{Category, SettingDescriptor, SettingsAppExt};
    const FIXTURE: SettingDescriptor = SettingDescriptor::new(
        "fixture.wheel_sensitivity",
        SettingKind::Int {
            min: 1,
            max: 10,
            step: 1,
        },
        "5",
        Category::Controls,
        5,
        "fixture-label",
        "fixture-desc",
    );
    let (mut app, _, db) = page_app("acceptance", "controls");
    app.register_setting(FIXTURE);
    {
        let mut loc = app
            .world_mut()
            .resource_mut::<opts_gui::l10n::Localization>();
        loc.extend(
            "en",
            "fixture-label = Wheel sensitivity\nfixture-desc = How strongly the wheel steers.\n",
        );
        loc.extend(
            "de",
            "fixture-label = Lenkradempfindlichkeit\nfixture-desc = Wie stark das Lenkrad lenkt.\n",
        );
    }
    app.world_mut().resource_mut::<SettingsPageState>().dirty = true;
    for _ in 0..3 {
        app.update();
    }
    // UI: the row exists and its control works.
    let up =
        by_id(&mut app, "settings/controls/fixture.wheel_sensitivity/up").expect("row in the UI");
    click(&mut app, up);
    app.update();
    app.update();
    assert_eq!(text(&app, "fixture.wheel_sensitivity"), "6");
    // Persistence.
    assert_eq!(
        stored(&db, "fixture.wheel_sensitivity").as_deref(),
        Some("6")
    );
    // Protocol.
    let listed = outcome_text(crate::settings_registry::protocol::list(
        app.world_mut(),
        &serde_json::json!({ "category": "controls" }),
    ));
    assert!(listed.contains("fixture.wheel_sensitivity"), "{listed}");
    // Generated reference page.
    let md = crate::settings_registry::reference::markdown_with(
        app.world().resource::<SettingsRegistry>(),
        app.world().resource::<opts_gui::l10n::Localization>(),
    );
    assert!(
        md.contains("`fixture.wheel_sensitivity`") && md.contains("Wheel sensitivity"),
        "{}",
        md.lines()
            .filter(|l| l.contains("fixture"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    // Search.
    let loc = app
        .world()
        .resource::<opts_gui::l10n::Localization>()
        .clone();
    let found = crate::settings_registry::search::search(
        app.world().resource::<SettingsRegistry>(),
        &loc,
        "wheel",
    );
    assert!(found.iter().any(|d| d.key == "fixture.wheel_sensitivity"));
}
