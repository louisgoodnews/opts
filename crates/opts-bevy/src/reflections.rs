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

mod flags;
mod notes;
mod stats;
mod wet;

pub use flags::{switches, ReflectionSwitches};
pub use notes::{note_material, MaterialNote};
pub use wet::{material_wetness, wet_objects_on, wet_standard, Wetness, WetnessHold};

use bevy::pbr::ScreenSpaceReflections;
use bevy::prelude::*;

use stats::{collect_stats, FrameStats};
use wet::{apply_wet, WetState};
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
        let wet0 = std::env::var("OPTS_WETNESS")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.0);
        // The wet pass is not a prototype any more: objects take it through the setting.
        app.insert_resource(sw.clone())
            .insert_resource(Wetness(wet0.clamp(0.0, 1.0)))
            .init_resource::<WetnessHold>()
            .init_resource::<WetState>()
            .add_systems(
                Update,
                apply_wet.run_if(resource_exists::<Assets<crate::ground_material::GroundMaterial>>),
            );
        if !sw.any() && std::env::var_os("OPTS_REFLECT_STATS").is_none() {
            return;
        }
        app.init_resource::<FrameStats>();
        if sw.ssr {
            // SSR only reflects what is rendered by the deferred path.
            app.insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::deferred());
        }
        if std::env::var_os("OPTS_REFLECT_GPU").is_some() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        }
        app.add_systems(Update, (apply_camera_features, collect_stats));
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
#[cfg(test)]
mod tests;
