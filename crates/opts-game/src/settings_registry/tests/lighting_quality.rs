//! The lighting-quality settings.

use super::*;
use opts_bevy::light_quality::{
    ladder_preset, GovernorState, LightMode, LightQuality, LightQualityInput, Shafts,
};

fn app() -> App {
    crate::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.insert_resource(SettingsRegistry::with_builtin())
        .insert_resource(SettingsCapabilities::default())
        .insert_resource(GameSettings::default())
        .init_resource::<Time<bevy::time::Real>>()
        .add_message::<SettingChanged>()
        .add_plugins(lighting::LightQualityGamePlugin);
    app.update();
    app
}

fn set(app: &mut App, key: &str, value: &str) {
    apply_change(app.world_mut(), key, Some(value), ChangeSource::Protocol)
        .unwrap_or_else(|e| panic!("{key}={value}: {e:?}"));
}

fn quality(app: &App) -> LightQuality {
    *app.world().resource::<LightQuality>()
}

#[test]
fn defaults_are_level_zero_in_automatic_mode() {
    let r = SettingsRegistry::with_builtin();
    let settings = GameSettings::default();
    let want = [
        ("graphics.light_quality_mode", "automatic"),
        ("graphics.light_shafts", "medium"),
        ("graphics.light_night_shafts", "true"),
        ("graphics.light_lantern_cones", "true"),
        ("graphics.light_headlight_cones", "true"),
        ("graphics.light_cone_cap", "64"),
        ("graphics.light_cone_depth_fade", "true"),
        ("graphics.mirror_count", "2"),
        ("graphics.mirror_rate", "2"),
        ("graphics.glass_reflections", "true"),
        ("graphics.bloom", "false"),
        ("graphics.light_governor_target_fps", "60"),
    ];
    for (key, default) in want {
        let d = r.get(key).unwrap_or_else(|| panic!("{key} missing"));
        assert_eq!(d.default, default, "{key}");
        assert_eq!(d.apply, Apply::Live, "{key} applies live");
        assert!(!d.hidden && d.category == Category::Graphics);
    }
    let custom = lighting::custom_of(&settings);
    assert!(custom.same_values(&ladder_preset(0)), "{custom:?}");
    assert_eq!(lighting::mode_of(&settings), LightMode::Automatic);
}

#[test]
fn every_custom_key_is_registered_and_the_texts_round_trip() {
    let r = SettingsRegistry::with_builtin();
    let mut settings = GameSettings::default();
    for level in 0..=5 {
        let q = ladder_preset(level);
        for (key, text) in lighting::CUSTOM_KEYS.iter().zip(lighting::texts_of(&q)) {
            assert!(r.get(key).is_some(), "{key}");
            r.set(&mut settings, &SettingsCapabilities::default(), key, &text)
                .unwrap_or_else(|e| panic!("{key}={text}: {e:?}"));
        }
        assert!(
            lighting::custom_of(&settings).same_values(&q),
            "level {level}"
        );
    }
    let caps = SettingsCapabilities::default();
    assert!(r
        .set(&mut settings, &caps, "graphics.light_cone_cap", "97")
        .is_err());
    assert!(r
        .set(&mut settings, &caps, "graphics.mirror_count", "3")
        .is_err());
    assert!(r
        .set(&mut settings, &caps, "graphics.mirror_rate", "3")
        .is_err());
    assert!(r
        .set(&mut settings, &caps, "graphics.light_quality_mode", "x")
        .is_err());
    assert!(r
        .set(
            &mut settings,
            &caps,
            "graphics.light_governor_target_fps",
            "29"
        )
        .is_err());
    assert!(r
        .set(
            &mut settings,
            &caps,
            "graphics.light_governor_target_fps",
            "241"
        )
        .is_err());
    r.set(
        &mut settings,
        &caps,
        "graphics.light_governor_target_fps",
        "144",
    )
    .expect("144 is in range");
    assert_eq!(lighting::target_fps_of(&settings), 144);
    r.set(&mut settings, &caps, "graphics.light_cone_cap", "96")
        .expect("96 is the maximum");
    assert_eq!(lighting::custom_of(&settings).cone_cap, 96);
}

#[test]
fn custom_mode_applies_the_controls_live() {
    let mut app = app();
    assert_eq!(quality(&app), ladder_preset(0));
    // Automatic: the controls do not change what is applied.
    set(&mut app, "graphics.bloom", "true");
    app.update();
    assert!(!quality(&app).bloom);
    // Custom: they are authoritative.
    set(&mut app, "graphics.light_quality_mode", "custom");
    app.update();
    app.update();
    let q = quality(&app);
    assert_eq!(q.mode, LightMode::Custom);
    assert!(
        !q.bloom,
        "the switch copied the automatic level over the old custom values"
    );
    set(&mut app, "graphics.bloom", "true");
    set(&mut app, "graphics.light_shafts", "ultra");
    set(&mut app, "graphics.light_cone_cap", "96");
    set(&mut app, "graphics.mirror_rate", "4");
    app.update();
    app.update();
    let q = quality(&app);
    assert_eq!(
        (q.shafts, q.cone_cap, q.mirror_every),
        (Shafts::Ultra, 96, 4)
    );
    assert_eq!(
        app.world().resource::<LightQualityInput>().mode,
        LightMode::Custom
    );
}

#[test]
fn automatic_to_custom_copies_the_level_reached() {
    let mut app = app();
    app.world_mut().resource_mut::<GovernorState>().level = 2;
    app.update();
    assert_eq!(quality(&app), ladder_preset(2));
    set(&mut app, "graphics.light_quality_mode", "custom");
    for _ in 0..3 {
        app.update();
    }
    let q = quality(&app);
    assert_eq!((q.mode, q.level), (LightMode::Custom, 2));
    assert!(q.same_values(&ladder_preset(2)), "{q:?}");
    let settings = app.world().resource::<GameSettings>();
    assert!(lighting::custom_of(settings).same_values(&ladder_preset(2)));
    assert_eq!(
        settings.raw("graphics.light_cone_cap").as_deref(),
        Some("16")
    );
    assert_eq!(
        settings.raw("graphics.light_lantern_cones").as_deref(),
        Some("false")
    );
}

#[test]
fn custom_to_automatic_restarts_at_level_zero() {
    let mut app = app();
    app.world_mut().resource_mut::<GovernorState>().level = 4;
    app.update();
    set(&mut app, "graphics.light_quality_mode", "custom");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(quality(&app).level, 4);
    set(&mut app, "graphics.light_quality_mode", "automatic");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(quality(&app), ladder_preset(0));
    assert_eq!(app.world().resource::<GovernorState>().level, 0);
}

#[test]
fn the_gate_is_open_only_in_a_running_loaded_drive() {
    use crate::app_state::AppState;
    use crate::in_game::types::{LoadingScreenRoot, SimulationLoadState};
    use opts_bevy::light_quality::GovernorGate;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_state::<AppState>()
        .init_resource::<GovernorGate>()
        .init_resource::<SimulationLoadState>()
        .add_systems(Update, lighting::update_governor_gate);
    let open = |app: &mut App| {
        app.update();
        app.world().resource::<GovernorGate>().open
    };
    assert!(!open(&mut app), "not in a drive");
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();
    assert!(!open(&mut app), "the drive is still loading");
    app.world_mut()
        .resource_mut::<SimulationLoadState>()
        .is_complete = true;
    assert!(open(&mut app), "loaded and running");
    let overlay = app.world_mut().spawn(LoadingScreenRoot).id();
    assert!(!open(&mut app), "the loading overlay is up");
    app.world_mut().despawn(overlay);
    assert!(open(&mut app));
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    assert!(!open(&mut app), "paused");
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    assert!(open(&mut app));
}
