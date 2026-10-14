//! Glass and other alpha-scaled parts of scenery objects (`[alphascale]`).
//!
//! The scenery runtime knows one live variable of this kind: `Envir_Brightness`, from the session
//! clock ([`SceneryClock::envir_brightness`]). A part scaled by any other variable stays hidden
//! (the variable is a script output the object's own scripts would have to raise; rain and dirt
//! overlays are not built at all, see [`crate::model::is_static_overlay`]).

use bevy::prelude::*;

use super::SceneryClock;
use crate::vehicle_lights::ALPHA_VISIBLE_MIN;

/// A scenery mesh part whose material alpha follows `[alphascale]`.
#[derive(Component, Debug, Clone)]
pub struct SceneryAlphaScale {
    /// Whether the variable is `Envir_Brightness` (else the part stays hidden).
    pub brightness: bool,
    /// The material is this part's own copy.
    pub private: bool,
    /// Alpha of the material before scaling.
    pub base_alpha: f32,
    /// Last applied factor (`NaN` before the first update).
    pub applied: f32,
}

impl SceneryAlphaScale {
    /// For a part scaled by `variable`.
    #[must_use]
    pub fn new(variable: &str) -> Self {
        Self {
            brightness: variable.trim().eq_ignore_ascii_case("envir_brightness"),
            private: false,
            base_alpha: f32::NAN,
            applied: f32::NAN,
        }
    }
}

/// Drives [`SceneryAlphaScale`] parts from the clock.
pub fn scenery_alpha_scale_system(
    clock: Res<SceneryClock>,
    mut parts: Query<(
        &mut SceneryAlphaScale,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let brightness = clock.envir_brightness();
    for (mut part, mut material, mut visibility) in &mut parts {
        let f = if part.brightness { brightness } else { 0.0 };
        if f == part.applied {
            continue;
        }
        let visible = f > ALPHA_VISIBLE_MIN;
        let want = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        if visible {
            if !part.private {
                let Some(copy) = materials.get(&material.0).cloned() else {
                    continue;
                };
                part.base_alpha = copy.base_color.alpha();
                material.0 = materials.add(copy);
                part.private = true;
            }
            let base = part.base_alpha;
            if let Some(mut m) = materials.get_mut(&material.0) {
                if !matches!(m.alpha_mode, AlphaMode::Mask(_)) {
                    m.base_color.set_alpha(base * f);
                }
            }
        }
        part.applied = f;
    }
}

/// Marks the spawned parts that follow an alpha variable: hidden until the system shows them.
pub fn bind_alpha_scale(commands: &mut Commands, spawned: &[Entity], parts: &[crate::ModelPart]) {
    for (e, part) in spawned.iter().zip(parts) {
        if let Some(var) = &part.alpha_variable {
            commands
                .entity(*e)
                .insert((SceneryAlphaScale::new(var), Visibility::Hidden));
        }
    }
}
