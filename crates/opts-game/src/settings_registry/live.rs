//! Effects of settings that are not read by a typed consumer: texture filtering, look, language,
//! log level and the developer panel.

use bevy::prelude::*;
use opts_gui::l10n::{Localization, LOCALE_ENV};

use super::SettingChanged;
use crate::in_game::tile_loading;
use crate::look::{ActiveLook, Look};
use crate::settings::GameSettings;

/// The crates whose log level the *Log detail* setting moves.
const LOG_TARGETS: [&str; 6] = [
    "opts_game",
    "opts_bevy",
    "opts_core",
    "opts_sim",
    "opts_gui",
    "opts_catalogue",
];

fn level_of(text: &str) -> tracing::Level {
    match text {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "debug" => tracing::Level::DEBUG,
        _ => tracing::Level::INFO,
    }
}

/// Applies a changed setting to the running game.
pub fn apply_live_changes(
    mut changes: MessageReader<SettingChanged>,
    mut loc: ResMut<Localization>,
    active: Option<ResMut<ActiveLook>>,
    overlay: Option<ResMut<crate::control_trace::TraceOverlay>>,
) {
    let mut active = active;
    let mut overlay = overlay;
    for c in changes.read() {
        match c.key.as_str() {
            "texture_filtering" => {
                if let Some(f) = opts_bevy::TextureFiltering::parse(&c.new) {
                    opts_bevy::set_texture_filtering(f);
                }
            }
            "look" => {
                if let (Some(a), Some(l)) = (active.as_mut(), Look::parse(&c.new)) {
                    a.0 = l;
                }
            }
            "interface.language" => loc.set_locale(&c.new),
            "advanced.trace_scene" => {
                if let Some(mode) = opts_bevy::scene_trace::Mode::parse(&c.new) {
                    opts_bevy::scene_trace::set_mode(mode);
                }
            }
            "advanced.trace_overlay" => {
                if let Some(o) = overlay.as_mut() {
                    o.enabled = c.new == "true";
                }
            }
            "advanced.log_level" => {
                for t in LOG_TARGETS {
                    let _ = opts_logging::set_level(t, level_of(&c.new));
                }
            }
            _ => {}
        }
    }
}

/// Start-up: the saved language (unless `OPTS_LOCALE` chooses one for this run).
pub fn apply_saved_language(settings: Res<GameSettings>, mut loc: ResMut<Localization>) {
    if std::env::var_os(LOCALE_ENV).is_some() {
        return;
    }
    if let Some(lang) = settings.raw("interface.language").filter(|l| !l.is_empty()) {
        if lang != loc.locale() && loc.can_select(&lang) {
            loc.set_locale(&lang);
        }
    }
}

/// The developer panel (the F10 block of the drive view) follows the setting; F10 still toggles
/// it for the moment.
pub fn sync_developer_panel(
    settings: Res<GameSettings>,
    stats: Option<ResMut<crate::in_game::omsi_world::WorldStats>>,
    mut last: Local<Option<bool>>,
) {
    let Some(mut stats) = stats else { return };
    let want = settings.raw("advanced.developer_panel").as_deref() == Some("true");
    if stats.is_added() || *last != Some(want) {
        stats.overlay_visible = want;
        *last = Some(want);
    }
}

/// Whether tiles built from now on draw their water (`graphics.water`).
pub fn sync_water(
    settings: Res<GameSettings>,
    job: Option<ResMut<crate::in_game::omsi_world::WorldLoadJob>>,
) {
    let Some(mut job) = job else { return };
    let want = settings.raw("graphics.water").as_deref() != Some("false");
    if job.water_enabled != want {
        job.water_enabled = want;
    }
}

/// Applies `gameplay.load_all_tiles` and `gameplay.tile_radius` to the streamer (M-0080): the
/// mode follows the settings and, for *automatic*, the hardware profile; the streamer loads what
/// the new mode wants and, in radius mode, unloads what is too far.
pub fn sync_tile_loading(
    settings: Res<GameSettings>,
    hardware: Option<Res<crate::hardware::HardwareState>>,
    job: Option<ResMut<crate::in_game::omsi_world::WorldLoadJob>>,
) {
    let Some(mut job) = job else { return };
    let hardware_changed = hardware.as_ref().is_some_and(|h| h.is_changed());
    if !(settings.is_changed() || hardware_changed || job.is_added()) {
        return;
    }
    let profile = hardware.as_ref().and_then(|h| h.profile.as_ref());
    let mode = tile_loading::resolve(
        tile_loading::TileChoice::from_settings(&settings),
        profile,
        job.map_radius(),
    );
    if job.tile_mode() != mode {
        job.set_tile_mode(mode);
    }
    let budget = tile_loading::stream_budget_of(&settings);
    if job.stream_budget != budget {
        job.stream_budget = budget;
    }
}

/// Metres of draw distance per step of `graphics.view_distance`.
pub const VIEW_DISTANCE_STEP_M: f32 = 150.0;
/// The step that means *unlimited* (the largest one).
pub const VIEW_DISTANCE_UNLIMITED: i64 = 8;

/// The scenery draw distance for a `graphics.view_distance` value: 150 m per step (the default 4 is
/// 600 m, beyond the default 3 x 3 tiles); the largest step draws everything that is loaded. A
/// value that is not a number counts as the default, one outside 1 to 8 is clamped.
#[must_use]
pub fn draw_distance_of(settings: &GameSettings) -> Option<f32> {
    let step = settings
        .raw("graphics.view_distance")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(4);
    let step = step.clamp(1, VIEW_DISTANCE_UNLIMITED);
    (step < VIEW_DISTANCE_UNLIMITED).then_some(step as f32 * VIEW_DISTANCE_STEP_M)
}

/// `graphics.view_distance`: how far scenery objects are drawn (M-0029).
pub fn sync_view_distance(
    settings: Res<GameSettings>,
    distance: Option<ResMut<opts_bevy::SceneryDrawDistance>>,
) {
    let Some(mut distance) = distance else { return };
    let want = draw_distance_of(&settings);
    if distance.metres != want {
        distance.metres = want;
    }
}

/// The volume settings (`audio.*`, percent) into the sound engine's [`opts_bevy::AudioSettings`]; master 0
/// switches the sound off, which also frees every voice.
pub fn sync_audio(settings: Res<GameSettings>, audio: Option<ResMut<opts_bevy::AudioSettings>>) {
    let Some(mut audio) = audio else { return };
    if !(settings.is_changed() || audio.is_added()) {
        return;
    }
    let percent = |key: &str, default: f32| -> f32 {
        settings
            .raw(key)
            .and_then(|v| v.parse::<f32>().ok())
            .map_or(default, |v| v.clamp(0.0, 100.0))
    };
    let want = opts_bevy::AudioSettings {
        enabled: percent("audio.master", 80.0) > 0.0,
        master: percent("audio.master", 80.0),
        effects: percent("audio.effects", 80.0),
        ambient: percent("audio.ambience", 60.0),
        vehicles: percent("audio.engine", 80.0),
        ui: percent("audio.interface", 60.0),
        max_voices: audio.max_voices,
    };
    if *audio != want {
        *audio = want;
    }
}

/// `graphics.hdr`: the main camera carries [`bevy::camera::Hdr`] while the setting is on (the
/// default). Off keeps the 8-bit main texture (saves about 50 MB at 1080p with 4x MSAA).
pub fn sync_hdr(
    settings: Res<GameSettings>,
    cameras: Query<(Entity, Has<bevy::camera::Hdr>), With<opts_bevy::MainCamera>>,
    mut commands: Commands,
) {
    let want = hdr_enabled(&settings);
    for (entity, has) in &cameras {
        if want && !has {
            commands.entity(entity).insert(bevy::camera::Hdr);
        } else if !want && has {
            commands.entity(entity).remove::<bevy::camera::Hdr>();
        }
    }
}

/// Whether `graphics.hdr` is on (the default).
#[must_use]
pub fn hdr_enabled(settings: &GameSettings) -> bool {
    settings.raw("graphics.hdr").as_deref() != Some("false")
}

/// The weather effects the settings ask for (`graphics.weather_*`).
#[must_use]
pub fn weather_effects_of(settings: &GameSettings) -> opts_bevy::WeatherEffects {
    let on = |key: &str| settings.raw(key).as_deref() != Some("false");
    opts_bevy::WeatherEffects {
        fog: on("graphics.weather_fog"),
        clouds: on("graphics.weather_clouds"),
        precipitation: settings
            .raw("graphics.weather_precipitation")
            .and_then(|q| opts_bevy::weather::PrecipitationQuality::parse(&q))
            .unwrap_or_default(),
        wet_ground: on("graphics.weather_wet_ground"),
    }
}

/// Keeps [`opts_bevy::WeatherEffects`] equal to the settings, live.
pub fn sync_weather_effects(
    settings: Res<GameSettings>,
    effects: Option<ResMut<opts_bevy::WeatherEffects>>,
) {
    let Some(mut effects) = effects else { return };
    let want = weather_effects_of(&settings);
    if *effects != want {
        *effects = want;
    }
}

/// Start-up: the saved scene trace mode (unless `--trace-scene` chose one for this run) and overlay.
pub fn apply_saved_trace(
    settings: Res<GameSettings>,
    args: Option<Res<crate::app::AppArgs>>,
    overlay: Option<ResMut<crate::control_trace::TraceOverlay>>,
) {
    if args.is_none_or(|a| a.0.trace_scene.is_none()) {
        if let Some(mode) = settings
            .raw("advanced.trace_scene")
            .and_then(|m| opts_bevy::scene_trace::Mode::parse(&m))
        {
            opts_bevy::scene_trace::set_mode(mode);
        }
    }
    if let Some(mut o) = overlay {
        o.enabled = settings.raw("advanced.trace_overlay").as_deref() == Some("true");
    }
}

#[cfg(test)]
mod view_distance_tests {
    use super::*;

    fn settings(value: Option<&str>) -> GameSettings {
        let mut s = GameSettings::default();
        if let Some(v) = value {
            s.set_raw("graphics.view_distance", v);
        }
        s
    }

    #[test]
    fn steps_are_150_metres_and_the_largest_is_unlimited() {
        assert_eq!(draw_distance_of(&settings(None)), Some(600.0));
        assert_eq!(draw_distance_of(&settings(Some("1"))), Some(150.0));
        assert_eq!(draw_distance_of(&settings(Some("7"))), Some(1050.0));
        assert_eq!(draw_distance_of(&settings(Some("8"))), None);
        assert_eq!(draw_distance_of(&settings(Some("0"))), Some(150.0));
        assert_eq!(draw_distance_of(&settings(Some("99"))), None);
        assert_eq!(draw_distance_of(&settings(Some("far"))), Some(600.0));
    }

    #[test]
    fn the_setting_drives_the_scenery_draw_distance() {
        let mut app = App::new();
        app.insert_resource(settings(Some("2")))
            .init_resource::<opts_bevy::SceneryDrawDistance>()
            .add_systems(Update, sync_view_distance);
        app.update();
        assert_eq!(
            app.world()
                .resource::<opts_bevy::SceneryDrawDistance>()
                .metres,
            Some(300.0)
        );
        app.world_mut()
            .resource_mut::<GameSettings>()
            .set_raw("graphics.view_distance", "8");
        app.update();
        assert_eq!(
            app.world()
                .resource::<opts_bevy::SceneryDrawDistance>()
                .metres,
            None
        );
    }
}

#[cfg(test)]
mod audio_tests {
    use super::*;

    #[test]
    fn the_volume_settings_reach_the_engine() {
        let mut app = App::new();
        let mut settings = GameSettings::default();
        settings.set_raw("audio.master", "50");
        settings.set_raw("audio.engine", "25");
        settings.set_raw("audio.ambience", "not a number");
        app.insert_resource(settings)
            .init_resource::<opts_bevy::AudioSettings>()
            .add_systems(Update, sync_audio);
        app.update();
        let a = app.world().resource::<opts_bevy::AudioSettings>().clone();
        assert_eq!(
            (a.master, a.vehicles, a.ui, a.effects),
            (50.0, 25.0, 60.0, 80.0)
        );
        assert_eq!(a.ambient, 60.0, "an unreadable value is the default");
        assert!(a.enabled);
        app.world_mut()
            .resource_mut::<GameSettings>()
            .set_raw("audio.master", "0");
        app.update();
        assert!(!app.world().resource::<opts_bevy::AudioSettings>().enabled);
    }
}
