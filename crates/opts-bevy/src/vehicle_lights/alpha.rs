//! Scales the alpha of glass and similar parts by the owners' script variables.

use super::*;

/// The alpha of a scaled part: its texture alpha times the owner's variable factor, the factor
/// clamped to `0..=1`. A factor of 0 gives alpha 0 (the part is hidden, see [`ALPHA_VISIBLE_MIN`]).
#[must_use]
pub fn scaled_alpha(base: f32, factor: f32) -> f32 {
    base * factor.clamp(0.0, 1.0)
}

/// Scales the alpha of glass and similar parts by the owners' script variables.
///
/// An owner without variables (a parked car: a vehicle model placed as scenery, no scripts, no
/// AI table) is static: `Envir_Brightness` comes from the session clock, as for scenery objects
/// ([`crate::scenery_runtime::SceneryAlphaScale`]), and any other variable is 0. Its parts waited
/// for variables that never came and stayed hidden: the TH_Wald parked cars, whose bodies are
/// scaled by `Envir_Brightness`, showed only wheels and windows.
pub fn vehicle_alpha_scale_system(
    mut parts: Query<(
        &mut AlphaScaleBinding,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
        Option<&crate::vehicle_anim::MeshAnimation>,
    )>,
    vehicles: Query<(Option<&VehicleScripts>, Option<&AiVariables>)>,
    clock: Option<Res<crate::SceneryClock>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut binding, mut material, mut visibility, animation) in &mut parts {
        let Ok((scripts, table)) = vehicles.get(binding.owner) else {
            continue;
        };
        let f = match VariableSource::of(scripts, table) {
            Some(engine) => engine.get(&binding.variable),
            None if binding
                .variable
                .trim()
                .eq_ignore_ascii_case("envir_brightness") =>
            {
                let Some(clock) = clock.as_deref() else {
                    continue;
                };
                clock.envir_brightness()
            }
            None => 0.0,
        }
        .clamp(0.0, 1.0);
        if f == binding.applied {
            continue;
        }
        // A part with its own `[visible]` rule has its visibility set by the animation system;
        // there the alpha alone carries the variable.
        if animation.is_none_or(|a| a.visible.is_empty()) {
            let want = if f > ALPHA_VISIBLE_MIN {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *visibility != want {
                *visibility = want;
            }
        }
        if f > ALPHA_VISIBLE_MIN || animation.is_some_and(|a| !a.visible.is_empty()) {
            if !binding.private {
                let Some(copy) = materials.get(&material.0).cloned() else {
                    continue;
                };
                binding.base_alpha = copy.base_color.alpha();
                material.0 = materials.add(copy);
                binding.private = true;
            }
            let base = binding.base_alpha;
            if let Some(mut m) = materials.get_mut(&material.0) {
                if !matches!(m.alpha_mode, AlphaMode::Mask(_)) {
                    m.base_color.set_alpha(scaled_alpha(base, f));
                }
            }
        }
        binding.applied = f;
    }
}
