//! Render quality settings: audio volumes, HDR, tree wind and the weather effects.

use bevy::prelude::*;

use crate::settings::GameSettings;

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

/// `graphics.tree_wind`: whether `[tree]` billboards move in the wind (M-0144; on by default).
pub fn sync_tree_wind(
    settings: Res<GameSettings>,
    state: Option<ResMut<opts_bevy::tree_wind::TreeWindState>>,
) {
    let Some(mut state) = state else { return };
    let on = settings.raw("graphics.tree_wind").as_deref() != Some("false");
    if state.enabled != on {
        state.enabled = on;
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
    let percent = |key: &str| {
        settings
            .raw(key)
            .and_then(|v| v.trim().parse::<u16>().ok())
            .map_or(100, |v| v.min(200))
    };
    opts_bevy::WeatherEffects {
        fog: on("graphics.weather_fog"),
        clouds: on("graphics.weather_clouds"),
        precipitation: settings
            .raw("graphics.weather_precipitation")
            .and_then(|q| opts_bevy::weather::PrecipitationQuality::parse(&q))
            .unwrap_or_default(),
        wet_ground: on("graphics.weather_wet_ground"),
        wet_objects: false,
        rain_percent: percent("graphics.rain_intensity"),
        snow_percent: percent("graphics.snow_intensity"),
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
