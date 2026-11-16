//! Wet standard materials: the [`Wetness`] driver and the pass that darkens and smooths the
//! opaque materials the loader noted.

use bevy::pbr::StandardMaterial;
use bevy::prelude::*;

use crate::ground_material::GroundMaterial;
use crate::texture::{texture_wetness, TextureFlags};
use crate::WeatherEffects;

use super::flags::ReflectionSwitches;
use super::notes::note_of;
use super::vehicle_wet::part_wetness;

/// How wet the world is, 0 (dry) to 1 (soaked). The single driver of the wet-surface parameters;
/// a weather system would write it.
#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct Wetness(pub f32);

/// A wetness held by the control protocol (`sim.set_time hold=true`, since 1.35). While `Some`,
/// the weather's ramp does not write [`Wetness`]: the drive keeps the held value until it is
/// released (`None`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Resource)]
pub struct WetnessHold(pub Option<f32>);

// ---------------------------------------------------------------------------------------------
// Wet surfaces
// ---------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
pub(super) struct WetState {
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

/// Whether the wet pass is on: the `wet` prototype switch, or the setting that wets objects and
/// vehicles (`WeatherEffects::wet_objects`).
#[must_use]
pub fn wet_objects_on(sw: &ReflectionSwitches, effects: Option<&WeatherEffects>) -> bool {
    sw.wet || effects.is_some_and(|e| e.wet_objects)
}

/// The wetness of one material: the global wetness when its texture has `[moisture]`, dry
/// otherwise (through [`texture_wetness`], the one rule for objects and vehicles).
#[must_use]
pub fn material_wetness(moisture: bool, w: f32) -> f32 {
    texture_wetness(
        TextureFlags {
            moisture,
            ..TextureFlags::default()
        },
        w,
    )
}

/// Applies the wet look to the noted opaque materials. Off (or dry) restores their original
/// colour and roughness exactly, so turning the setting off leaves objects dry again.
pub(super) fn apply_wet(
    sw: Res<ReflectionSwitches>,
    wetness: Res<Wetness>,
    effects: Option<Res<WeatherEffects>>,
    mut state: ResMut<WetState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    ground: Res<Assets<GroundMaterial>>,
) {
    let w = if wet_objects_on(&sw, effects.as_deref()) {
        wetness.0
    } else {
        0.0
    };
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
        let wv = part_wetness(note.tyre, note.moisture, w);
        let (c, r) = if wv > 0.0 {
            wet_standard(note.base_color, note.roughness, wv)
        } else {
            (note.base_color, note.roughness)
        };
        if let Some(mut m) = materials.get_mut(id) {
            m.base_color = c;
            m.perceptual_roughness = r;
        }
    }
    // The layered ground follows `Wetness` too, but the weather writes its uniform (with puddles
    // and snow, `in_game::weather::surfaces`), so there is one implementation of wet ground.
}
