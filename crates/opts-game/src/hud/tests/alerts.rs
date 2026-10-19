//! Alerts, camera label, status bar, units and the developer panel.

use super::*;

#[test]
fn the_alert_row_has_a_fixed_minimum_height_and_the_alert_cannot_be_switched_off() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let fixed = app
        .world_mut()
        .query::<(&Node, &Children)>()
        .iter(app.world())
        .any(|(n, c)| {
            n.min_height == Val::Px(44.0)
                && c.iter()
                    .any(|e| app.world().get::<build::AlertPill>(e).is_some())
        });
    assert!(fixed, "the alert line sits in a box of fixed height");
    assert!(widget::widget("alerts").is_some_and(|w| w.switch.is_none()));
}

#[test]
fn an_alert_shows_in_the_slot_and_critical_stays() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut().write_message(RaiseAlert {
        id: "t",
        severity: AlertSeverity::Critical,
        key: keys::ALERT_TEST,
        args: Vec::new(),
    });
    app.update();
    app.update();
    let line = |app: &mut App| {
        app.world_mut()
            .query::<(&build::HudText, &Text)>()
            .iter(app.world())
            .find(|(h, _)| h.binding == Binding::Alerts)
            .map(|(_, t)| t.0.clone())
    };
    let text = line(&mut app).unwrap_or_default();
    assert!(text.starts_with("Alert:"), "{text}");
    let visible = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Visibility, With<build::AlertPill>>()
            .iter(app.world())
            .next()
            .copied()
    };
    assert_eq!(visible(&mut app), Some(Visibility::Inherited));
    app.world_mut().write_message(ClearAlert("t"));
    app.update();
    assert_eq!(visible(&mut app), Some(Visibility::Hidden));
}

#[test]
fn a_camera_change_shows_the_label_for_one_and_a_half_seconds() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let alpha = |app: &mut App| {
        app.world_mut()
            .query::<(&FadeKind, &HudFade)>()
            .iter(app.world())
            .find(|(k, _)| **k == FadeKind::CameraLabel)
            .map(|(_, f)| f.alpha(false))
    };
    assert_eq!(alpha(&mut app), Some(0.0), "no label at the start");
    assert!(app.world().resource::<CameraLabel>().shown.is_none());
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    app.update();
    app.update();
    assert_eq!(alpha(&mut app), Some(1.0));
    // advance the fade by hand: 1.6 s later it is gone
    for (k, mut f) in app
        .world_mut()
        .query::<(&FadeKind, &mut HudFade)>()
        .iter_mut(app.world_mut())
    {
        if *k == FadeKind::CameraLabel {
            f.elapsed = CAMERA_LABEL_SECONDS + 0.1;
        }
    }
    assert_eq!(alpha(&mut app), Some(0.0));
    let text = app
        .world_mut()
        .query::<(&build::HudText, &Text)>()
        .iter(app.world())
        .find(|(h, _)| h.binding == Binding::CameraLabel)
        .map(|(_, t)| t.0.clone());
    assert_eq!(text.as_deref(), Some("Driver's view"));
}

#[test]
fn the_status_bar_is_pinned_while_the_game_is_paused() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.update();
    let pinned = app
        .world_mut()
        .query::<(&FadeKind, &HudFade)>()
        .iter(app.world())
        .find(|(k, _)| **k == FadeKind::Status)
        .map(|(_, f)| f.pinned);
    assert_eq!(pinned, Some(true));
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    app.update();
    let f = app
        .world_mut()
        .query::<(&FadeKind, &HudFade)>()
        .iter(app.world())
        .find(|(k, _)| **k == FadeKind::Status)
        .map(|(_, f)| (f.pinned, f.elapsed));
    assert!(matches!(f, Some((false, e)) if e < 1.0));
}

#[test]
fn speed_follows_the_unit_setting() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut().resource_mut::<HudValues>().speed_kmh = 50.0;
    // `sample_values` would reset the values without a vehicle; read the texts right after
    // the update systems by running the text system alone.
    let speed = |app: &mut App| {
        app.world_mut()
            .run_system_once(update::update_texts)
            .expect("runs");
        app.world_mut()
            .query::<(&build::HudText, &Text)>()
            .iter(app.world())
            .filter(|(h, _)| h.binding == Binding::Speed)
            .map(|(_, t)| t.0.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(speed(&mut app), ["50", "KM/H"]);
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("interface.units", "mph");
    assert_eq!(speed(&mut app), ["31", "MPH"]);
}

#[test]
fn the_developer_panel_is_off_by_default_and_f10_changes_its_setting() {
    use crate::settings_registry::{SettingChanged, SettingsCapabilities, SettingsRegistry};
    let dir = std::env::temp_dir().join(format!("opts-hud-dev-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let mut settings = GameSettings::default();
    settings.db_path_override = Some(dir.join("settings.db"));
    let mut app = App::new();
    app.insert_resource(settings)
        .insert_resource(SettingsRegistry::with_builtin())
        .init_resource::<SettingsCapabilities>()
        .add_message::<SettingChanged>()
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(WorldStats::default())
        .add_systems(
            Update,
            (
                crate::in_game::omsi_world::toggle_developer_panel,
                crate::settings_registry::live::sync_developer_panel,
            )
                .chain(),
        );
    app.update();
    assert!(!app.world().resource::<WorldStats>().overlay_visible);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F10);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    assert_eq!(
        app.world()
            .resource::<GameSettings>()
            .raw("advanced.developer_panel")
            .as_deref(),
        Some("true")
    );
    app.update();
    assert!(app.world().resource::<WorldStats>().overlay_visible);
    let _ = std::fs::remove_dir_all(&dir);
}
