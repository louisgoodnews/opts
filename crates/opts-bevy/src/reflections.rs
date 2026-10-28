//! Reflections: the shared entry point of the sky cube map, the glass look and the mirror cameras
//! ([`ReflectionsPlugin`]), plus the remaining research prototypes (wet standard materials, SSR,
//! frame statistics).
//!
//! The normal path does not depend on any environment variable:
//!
//! * [`crate::sky_cube`] - one shared sky cube map as image-based light on the
//!   [`MainCamera`](crate::MainCamera), controlled by `LightQuality::glass_reflections`;
//! * [`crate::glass`] - glass materials get a low roughness and premultiplied alpha while that
//!   setting is on;
//! * [`crate::mirrors`] - one render-to-texture camera per mirror of the player's vehicle,
//!   controlled by `LightQuality::mirror_count` and `mirror_every`.
//!
//! The prototypes behind `OPTS_REFLECT` (comma separated, read once at start-up) that remain:
//!
//! | Switch | Effect |
//! | :--- | :--- |
//! | `wet` | [`Wetness`]-driven roughness drop and darkening of standard materials |
//! | `ssr` | screen-space reflections (switches the camera to deferred rendering, MSAA off) |
//!
//! `OPTS_WETNESS=0..1` sets the initial [`Wetness`]. `OPTS_REFLECT_STATS=<file>` makes the plugin
//! write frame-time statistics (after `OPTS_REFLECT_STATS_AFTER` seconds, default 20) and exit.
//!
//! See `docs/src/architecture/reflections-research.md` for what the research proves and what it
//! does not.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use bevy::pbr::{ScreenSpaceReflections, StandardMaterial};
use bevy::prelude::*;

use crate::ground_material::GroundMaterial;

/// The parsed `OPTS_REFLECT` switches (research prototypes only).
#[derive(Debug, Clone, Default, Resource)]
pub struct ReflectionSwitches {
    /// Wet-surface parameter set.
    pub wet: bool,
    /// Screen-space reflections (deferred).
    pub ssr: bool,
}

impl ReflectionSwitches {
    /// Parses a switch list such as `wet,ssr`; unknown items (the former `env`, `glass`, `mirror`
    /// switches are normal features now) are ignored.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for item in text.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            let key = item.split_once('=').map_or(item, |(k, _)| k);
            match key {
                "wet" => s.wet = true,
                "ssr" => s.ssr = true,
                _ => {}
            }
        }
        s
    }

    /// Reads `OPTS_REFLECT`; all off when unset.
    #[must_use]
    pub fn from_env() -> Self {
        Self::parse(&std::env::var("OPTS_REFLECT").unwrap_or_default())
    }

    /// Whether any switch is on.
    #[must_use]
    pub fn any(&self) -> bool {
        self.wet || self.ssr
    }
}

/// How wet the world is, 0 (dry) to 1 (soaked). The single driver of the wet-surface parameters;
/// a weather system would write it.
#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct Wetness(pub f32);

/// What the model loader recorded about one material, enough to treat it afterwards.
#[derive(Debug, Clone)]
pub struct MaterialNote {
    /// Texture file name of the material, lower case.
    pub texture: String,
    /// Original alpha mode.
    pub alpha: AlphaMode,
    /// Original base colour.
    pub base_color: Color,
    /// Original perceptual roughness.
    pub roughness: f32,
}

static SWITCHES: OnceLock<ReflectionSwitches> = OnceLock::new();
static NOTES: Mutex<Option<HashMap<AssetId<StandardMaterial>, MaterialNote>>> = Mutex::new(None);

/// The switches read from the environment once.
pub fn switches() -> &'static ReflectionSwitches {
    SWITCHES.get_or_init(ReflectionSwitches::from_env)
}

/// Called by the model loader for every material it creates; free when no switch is on.
pub fn note_material(
    handle: &Handle<StandardMaterial>,
    texture: Option<&str>,
    m: &StandardMaterial,
) {
    if !switches().any() {
        return;
    }
    let note = MaterialNote {
        texture: texture.unwrap_or("").to_lowercase(),
        alpha: m.alpha_mode,
        base_color: m.base_color,
        roughness: m.perceptual_roughness,
    };
    if let Ok(mut g) = NOTES.lock() {
        g.get_or_insert_with(HashMap::new).insert(handle.id(), note);
    }
}

fn note_of(id: AssetId<StandardMaterial>) -> Option<MaterialNote> {
    NOTES.lock().ok()?.as_ref()?.get(&id).cloned()
}

/// Marks the main camera once it carries the prototype components.
#[derive(Component)]
struct ReflectionsApplied;

/// The normal reflection features: the sky cube map, the glass look and the mirror cameras.
/// (The game feeds [`crate::sky_cube::SkyCubeInput`].)
#[derive(Default)]
pub struct ReflectionsPlugin;

impl Plugin for ReflectionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            crate::sky_cube::SkyCubePlugin,
            crate::glass::GlassReflectionsPlugin,
            crate::mirrors::MirrorPlugin,
        ));
    }
}

/// Registers the remaining prototypes; does nothing unless a switch is on.
pub struct ReflectionsResearchPlugin;

impl Plugin for ReflectionsResearchPlugin {
    fn build(&self, app: &mut App) {
        let sw = switches().clone();
        if !sw.any() && std::env::var_os("OPTS_REFLECT_STATS").is_none() {
            return;
        }
        let wet0 = std::env::var("OPTS_WETNESS")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.0);
        app.insert_resource(sw.clone())
            .insert_resource(Wetness(wet0.clamp(0.0, 1.0)))
            .init_resource::<WetState>()
            .init_resource::<FrameStats>();
        if sw.ssr {
            // SSR only reflects what is rendered by the deferred path.
            app.insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::deferred());
        }
        if std::env::var_os("OPTS_REFLECT_GPU").is_some() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        }
        app.add_systems(Update, (apply_camera_features, apply_wet, collect_stats));
    }
}

/// Encodes a non-negative radiance as an IEEE half float (round to nearest, tiny values flush to
/// zero, huge values clamp).
pub(crate) fn f16_bits(v: f32) -> u16 {
    let v = v.clamp(0.0, 65_000.0);
    if v < 6.1e-5 {
        return 0;
    }
    let bits = v.to_bits();
    let exp = ((bits >> 23) as i32) - 127 + 15;
    let man = (bits >> 13) & 0x3ff;
    let round = (bits >> 12) & 1;
    ((((exp as u32) << 10) | man) + round) as u16
}

fn apply_camera_features(
    sw: Res<ReflectionSwitches>,
    mut commands: Commands,
    cameras: Query<Entity, (With<crate::MainCamera>, Without<ReflectionsApplied>)>,
) {
    for e in &cameras {
        let mut c = commands.entity(e);
        c.insert(ReflectionsApplied);
        if sw.ssr {
            c.insert((
                Msaa::Off,
                ScreenSpaceReflections::default(),
                bevy::core_pipeline::prepass::DepthPrepass,
                bevy::core_pipeline::prepass::DeferredPrepass,
            ));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Wet surfaces
// ---------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct WetState {
    last: f32,
    applied_count: usize,
}

/// Roughness and colour of a standard material at wetness `w`.
#[must_use]
pub fn wet_standard(base: Color, roughness: f32, w: f32) -> (Color, f32) {
    let c = base.to_linear();
    let k = 1.0 - 0.35 * w;
    let wet = Color::linear_rgba(c.red * k, c.green * k, c.blue * k, c.alpha);
    (wet, roughness * (1.0 - 0.65 * w))
}

fn apply_wet(
    sw: Res<ReflectionSwitches>,
    wetness: Res<Wetness>,
    mut state: ResMut<WetState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    ground: Res<Assets<GroundMaterial>>,
) {
    if !sw.wet {
        return;
    }
    let w = wetness.0;
    let ground_count = ground.len();
    if (w - state.last).abs() < 1e-4 && state.applied_count == materials.len() + ground_count {
        return;
    }
    state.last = w;
    state.applied_count = materials.len() + ground_count;
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        let Some(note) = note_of(id) else { continue };
        if !matches!(note.alpha, AlphaMode::Opaque) {
            continue;
        }
        if let Some(mut m) = materials.get_mut(id) {
            let (c, r) = wet_standard(note.base_color, note.roughness, w);
            m.base_color = c;
            m.perceptual_roughness = r;
        }
    }
    // The layered ground follows `Wetness` too, but the weather writes its uniform (with puddles
    // and snow, `in_game::weather::surfaces`), so there is one implementation of wet ground.
}
// ---------------------------------------------------------------------------------------------
// Frame statistics
// ---------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct FrameStats {
    samples: Vec<f32>,
    started: Option<f64>,
}

fn rss_mib() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(0.0, |kb| kb / 1024.0)
}

fn collect_stats(
    time: Res<Time<Real>>,
    mut stats: ResMut<FrameStats>,
    diagnostics: Option<Res<bevy::diagnostic::DiagnosticsStore>>,
    mut exit: MessageWriter<AppExit>,
    cameras: Query<(), With<crate::MainCamera>>,
) {
    let Ok(path) = std::env::var("OPTS_REFLECT_STATS") else {
        return;
    };
    // Statistics start when the 3D scene is on screen (a camera exists and frames are short).
    if cameras.is_empty() {
        return;
    }
    let now = time.elapsed_secs_f64();
    let start = *stats.started.get_or_insert(now);
    let warmup: f64 = std::env::var("OPTS_REFLECT_WARMUP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8.0);
    let span: f64 = std::env::var("OPTS_REFLECT_STATS_AFTER")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20.0);
    if now - start > warmup {
        stats.samples.push(time.delta_secs() * 1000.0);
    }
    if now - start > warmup + span {
        let mut v = stats.samples.clone();
        v.sort_by(|a, b| a.total_cmp(b));
        let n = v.len().max(1);
        let mean = v.iter().sum::<f32>() / n as f32;
        let pct = |p: f32| {
            v.get(((n as f32 - 1.0) * p) as usize)
                .copied()
                .unwrap_or(0.0)
        };
        let mut out = format!(
            "frames={} mean_ms={mean:.3} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} fps={:.1} rss_mib={:.0}\n",
            v.len(),
            pct(0.5),
            pct(0.95),
            pct(0.99),
            1000.0 / mean.max(1e-3),
            rss_mib()
        );
        if let Some(d) = diagnostics {
            for diag in d.iter() {
                let p = diag.path().as_str();
                if p.starts_with("render/") || p.contains("elapsed_gpu") {
                    if let Some(avg) = diag.average() {
                        out.push_str(&format!("diag {p} avg={avg:.4}\n"));
                    }
                }
            }
        }
        let _ = std::fs::write(&path, out);
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests;
