//! The lighting-quality settings (`graphics.light_*`, `graphics.mirror_*`, `graphics.glass_reflections`,
//! `graphics.bloom`) and their link to [`opts_bevy::light_quality`]
//! (`docs/src/architecture/lighting-system.md` section 9).
//!
//! `graphics.light_quality_mode` chooses who decides: `automatic` (the governor, the controls
//! below show what it applies and are disabled) or `custom` (the controls are authoritative).

use bevy::prelude::*;
use opts_bevy::light_quality::{
    ladder_preset, GovernorGate, LightMode, LightQuality, LightQualityInput, LightQualitySet,
    Shafts,
};

use super::descriptor::{opt, Category as C, SettingDescriptor as D, SettingKind as K};
use super::{ChangeSource, SettingChanged, SettingsApi};
use crate::settings::GameSettings;

/// The mode setting.
pub const MODE_KEY: &str = "graphics.light_quality_mode";

const MODES: &[super::EnumOption] = &[
    opt("automatic", "set-opt-lightmode-automatic"),
    opt("custom", "set-opt-lightmode-custom"),
];

const SHAFTS: &[super::EnumOption] = &[
    opt("off", "set-opt-shafts-off"),
    opt("low", "set-opt-shafts-low"),
    opt("medium", "set-opt-shafts-medium"),
    opt("high", "set-opt-shafts-high"),
    opt("ultra", "set-opt-shafts-ultra"),
];

const MIRROR_RATES: &[super::EnumOption] = &[
    opt("1", "set-opt-mirror-rate-1"),
    opt("2", "set-opt-mirror-rate-2"),
    opt("4", "set-opt-mirror-rate-4"),
];

/// The custom controls (every key except the mode), in page order.
pub const CUSTOM_KEYS: [&str; 10] = [
    "graphics.light_shafts",
    "graphics.light_night_shafts",
    "graphics.light_lantern_cones",
    "graphics.light_headlight_cones",
    "graphics.light_cone_cap",
    "graphics.light_cone_depth_fade",
    "graphics.mirror_count",
    "graphics.mirror_rate",
    "graphics.glass_reflections",
    "graphics.bloom",
];

/// Whether `key` is one of the custom controls (disabled while the mode is automatic).
#[must_use]
pub fn is_custom_key(key: &str) -> bool {
    CUSTOM_KEYS.contains(&key)
}

/// Whether the stored mode is automatic (the default; an unknown text counts as automatic).
#[must_use]
pub fn is_automatic(settings: &GameSettings) -> bool {
    mode_of(settings) == LightMode::Automatic
}

/// The stored mode.
#[must_use]
pub fn mode_of(settings: &GameSettings) -> LightMode {
    settings
        .raw(MODE_KEY)
        .and_then(|m| LightMode::parse(&m))
        .unwrap_or_default()
}

fn d(
    key: &'static str,
    kind: K,
    default: &'static str,
    order: u16,
    label: &'static str,
    desc: &'static str,
    keywords: &'static [&'static str],
) -> D {
    D::new(key, kind, default, C::Graphics, order, label, desc)
        .keywords(keywords)
        .since("0.3")
}

/// The descriptors; registered by [`super::builtin::all`].
#[must_use]
pub fn descriptors() -> Vec<D> {
    let int = |min, max, step| K::Int { min, max, step };
    vec![
        d(
            MODE_KEY,
            K::Enum(MODES),
            "automatic",
            100,
            "set-light-mode",
            "set-light-mode-desc",
            &[
                "lighting",
                "quality",
                "governor",
                "light shafts",
                "performance",
                "auto",
            ],
        ),
        d(
            CUSTOM_KEYS[0],
            K::Enum(SHAFTS),
            "medium",
            101,
            "set-light-shafts",
            "set-light-shafts-desc",
            &["sun rays", "god rays", "volumetric", "fog", "steps"],
        ),
        d(
            CUSTOM_KEYS[1],
            K::Bool,
            "true",
            102,
            "set-light-night-shafts",
            "set-light-night-shafts-desc",
            &["moon", "night", "rays"],
        ),
        d(
            CUSTOM_KEYS[2],
            K::Bool,
            "true",
            103,
            "set-light-lantern-cones",
            "set-light-lantern-cones-desc",
            &["street lamps", "light cones", "lanterns"],
        ),
        d(
            CUSTOM_KEYS[3],
            K::Bool,
            "true",
            104,
            "set-light-headlight-cones",
            "set-light-headlight-cones-desc",
            &["headlights", "light cones", "vehicles"],
        ),
        d(
            CUSTOM_KEYS[4],
            int(0, 96, 8),
            "64",
            105,
            "set-light-cone-cap",
            "set-light-cone-cap-desc",
            &["light cones", "limit", "maximum"],
        ),
        d(
            CUSTOM_KEYS[5],
            K::Bool,
            "true",
            106,
            "set-light-cone-depth-fade",
            "set-light-cone-depth-fade-desc",
            &["light cones", "soft", "depth prepass"],
        ),
        d(
            CUSTOM_KEYS[6],
            int(0, 2, 1),
            "2",
            107,
            "set-mirror-count",
            "set-mirror-count-desc",
            &["mirrors", "rear view", "reflection"],
        ),
        d(
            CUSTOM_KEYS[7],
            K::Enum(MIRROR_RATES),
            "2",
            108,
            "set-mirror-rate",
            "set-mirror-rate-desc",
            &["mirrors", "update rate", "frames"],
        ),
        d(
            CUSTOM_KEYS[8],
            K::Bool,
            "true",
            109,
            "set-glass-reflections",
            "set-glass-reflections-desc",
            &["windows", "glass", "sky", "reflection"],
        ),
        d(
            CUSTOM_KEYS[9],
            K::Bool,
            "false",
            110,
            "set-bloom",
            "set-bloom-desc",
            &["glow", "hdr", "lights"],
        ),
    ]
}

/// The custom values the settings hold (`mode` and `level` of the result are placeholders).
#[must_use]
pub fn custom_of(settings: &GameSettings) -> LightQuality {
    let def = ladder_preset(0);
    let flag = |key: &str, default: bool| match settings.raw(key).as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => default,
    };
    let num = |key: &str, default: u32| {
        settings
            .raw(key)
            .and_then(|v| v.trim().parse::<u32>().ok())
            .unwrap_or(default)
    };
    LightQuality {
        mode: LightMode::Custom,
        level: 0,
        shafts: settings
            .raw(CUSTOM_KEYS[0])
            .and_then(|s| Shafts::parse(&s))
            .unwrap_or(def.shafts),
        night_shafts: flag(CUSTOM_KEYS[1], def.night_shafts),
        lantern_cones: flag(CUSTOM_KEYS[2], def.lantern_cones),
        headlight_cones: flag(CUSTOM_KEYS[3], def.headlight_cones),
        cone_cap: num(CUSTOM_KEYS[4], def.cone_cap),
        cone_depth_fade: flag(CUSTOM_KEYS[5], def.cone_depth_fade),
        mirror_count: num(CUSTOM_KEYS[6], u32::from(def.mirror_count)) as u8,
        mirror_every: num(CUSTOM_KEYS[7], u32::from(def.mirror_every)) as u8,
        glass_reflections: flag(CUSTOM_KEYS[8], def.glass_reflections),
        bloom: flag(CUSTOM_KEYS[9], def.bloom),
    }
    .clamped()
}

/// The stored text of each custom control for `q`, in [`CUSTOM_KEYS`] order.
#[must_use]
pub fn texts_of(q: &LightQuality) -> [String; 10] {
    [
        q.shafts.name().to_string(),
        q.night_shafts.to_string(),
        q.lantern_cones.to_string(),
        q.headlight_cones.to_string(),
        q.cone_cap.to_string(),
        q.cone_depth_fade.to_string(),
        q.mirror_count.to_string(),
        q.mirror_every.to_string(),
        q.glass_reflections.to_string(),
        q.bloom.to_string(),
    ]
}

/// The stored text of the custom control `key` for `q`.
#[must_use]
pub fn text_for(q: &LightQuality, key: &str) -> Option<String> {
    let i = CUSTOM_KEYS.iter().position(|k| *k == key)?;
    Some(texts_of(q)[i].clone())
}

/// Keeps [`LightQualityInput`] equal to the settings, live.
pub fn sync_light_input(settings: Res<GameSettings>, mut input: ResMut<LightQualityInput>) {
    let want = LightQualityInput {
        mode: mode_of(&settings),
        custom: custom_of(&settings),
    };
    if *input != want {
        *input = want;
    }
}

/// Set when the mode just changed from automatic to custom ([`copy_level_to_custom`] acts on it).
#[derive(Resource, Debug, Default)]
pub struct CopyLevelPending(bool);

/// Notices the switch from automatic to custom.
pub fn notice_switch_to_custom(
    mut changes: MessageReader<SettingChanged>,
    mut pending: ResMut<CopyLevelPending>,
) {
    if changes
        .read()
        .any(|c| c.key == MODE_KEY && c.old != "custom" && c.new == "custom")
    {
        pending.0 = true;
    }
}

/// Automatic to custom: "keeps the level reached as starting values" (the custom controls take
/// the values the governor applied). Custom to automatic needs nothing here: the governor
/// restarts at level 0 when it sees the mode change.
pub fn copy_level_to_custom(
    mut pending: ResMut<CopyLevelPending>,
    quality: Res<LightQuality>,
    mut api: SettingsApi,
) {
    if !std::mem::take(&mut pending.0) {
        return;
    }
    let level = ladder_preset(quality.level);
    for (key, text) in CUSTOM_KEYS.iter().zip(texts_of(&level)) {
        let _ = api.change(key, Some(&text), ChangeSource::Protocol, true);
    }
    tracing::info!(
        level = quality.level,
        "lighting quality: custom controls start from the automatic level"
    );
}

/// Opens the governor only in a drive that is running: not paused, not loading.
#[allow(clippy::type_complexity)]
pub fn update_governor_gate(
    state: Option<Res<State<crate::app_state::AppState>>>,
    time: Option<Res<Time<Virtual>>>,
    load: Option<Res<crate::in_game::types::SimulationLoadState>>,
    loading_screen: Query<(), With<crate::in_game::types::LoadingScreenRoot>>,
    rebuild: Option<Res<opts_bevy::ChronoRebuild>>,
    mut gate: ResMut<GovernorGate>,
) {
    let open = state.is_some_and(|s| *s.get() == crate::app_state::AppState::InGame)
        && time.is_some_and(|t| !t.is_paused())
        && load.is_none_or(|l| l.is_complete)
        && loading_screen.is_empty()
        && !rebuild.is_some_and(|r| r.is_pending());
    if gate.open != open {
        gate.open = open;
    }
}

/// Registers the lighting quality state, the governor and the glue to the settings.
pub struct LightQualityGamePlugin;

impl Plugin for LightQualityGamePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<opts_bevy::light_quality::LightQualityPlugin>() {
            app.add_plugins(opts_bevy::light_quality::LightQualityPlugin);
        }
        app.init_resource::<CopyLevelPending>().add_systems(
            Update,
            ((
                notice_switch_to_custom,
                copy_level_to_custom,
                sync_light_input,
                update_governor_gate,
            )
                .chain()
                .before(LightQualitySet),),
        );
    }
}
