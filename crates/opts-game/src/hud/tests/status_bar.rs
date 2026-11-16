//! Status bar pinning, speed units and the developer panel toggle of the HUD.

use super::*;

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
