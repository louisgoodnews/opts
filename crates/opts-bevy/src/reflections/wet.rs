//! Wet standard materials: the [`Wetness`] driver and the pass that darkens and smooths the
//! opaque materials the loader noted.

use bevy::pbr::StandardMaterial;
use bevy::prelude::*;

use crate::ground_material::GroundMaterial;

use super::flags::ReflectionSwitches;
use super::notes::note_of;

/// How wet the world is, 0 (dry) to 1 (soaked). The single driver of the wet-surface parameters;
/// a weather system would write it.
#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct Wetness(pub f32);

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

pub(super) fn apply_wet(
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
