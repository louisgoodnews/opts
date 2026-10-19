//! Widget switches, messages, focus and the cluster layout.

use super::*;

#[test]
fn every_widget_switch_is_a_registered_setting_with_matching_default() {
    let registry = crate::settings_registry::SettingsRegistry::with_builtin();
    for w in WIDGETS {
        let Some(sw) = w.switch else { continue };
        let d = registry
            .get(sw.key)
            .unwrap_or_else(|| panic!("{} missing", sw.key));
        assert_eq!(d.default == "true", sw.default_on, "{}", sw.key);
        assert_eq!(d.label_key, sw.label_key);
    }
}

#[test]
fn every_hud_message_exists_in_english_and_german() {
    let mut missing = Vec::new();
    for locale in ["en", "de"] {
        let mut loc = crate::settings_registry::localization(locale);
        add_messages(&mut loc);
        let mut all: Vec<&str> = keys::ALL.to_vec();
        for w in WIDGETS {
            all.push(w.title_key);
        }
        for k in all {
            if loc.t(k).starts_with('⟦') {
                missing.push(format!("{locale}:{k}"));
            }
        }
    }
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn the_hud_has_no_focusable_element_except_pointer_only_chrome() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<HudRoot>>()
        .iter(app.world())
        .next()
        .expect("root");
    let mut checked = 0;
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        let world = app.world();
        if world.get::<Focusable>(e).is_some() {
            assert!(world.get::<PointerOnly>(e).is_some(), "{e:?} takes focus");
        }
        checked += 1;
        if let Some(c) = world.get::<Children>(e) {
            stack.extend(c.iter());
        }
    }
    assert!(checked > 15, "walked the HUD ({checked} entities)");
}

#[test]
fn the_cluster_is_built_from_the_profile_and_rpm_is_hidden_without_a_value() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    for id in ["speed", "gear", "parking-brake"] {
        assert_eq!(display_of(&mut app, id), Some(Display::Flex), "{id}");
    }
    assert_eq!(display_of(&mut app, "rpm"), Some(Display::None));
}

#[test]
fn switching_a_widget_off_removes_it_and_an_empty_slot_leaves_the_layout() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("hud.gear", "false");
    app.update();
    assert_eq!(display_of(&mut app, "gear"), Some(Display::None));
    assert_eq!(display_of(&mut app, "speed"), Some(Display::Flex));
    for key in ["hud.speed", "hud.parking_brake"] {
        app.world_mut()
            .resource_mut::<GameSettings>()
            .set_raw(key, "false");
    }
    app.update();
    let cluster = app
        .world_mut()
        .query::<(&build::HudRow, &Node)>()
        .iter(app.world())
        .find(|(s, _)| s.0 == widget::Row::Instruments)
        .map(|(_, n)| n.display);
    assert_eq!(cluster, Some(Display::None));
}
