//! The brightness of the cones: what a source reads and what is applied to each cone per frame.

use super::*;

/// The brightness a cone reads from its source (`0..=1`).
pub(super) fn level_of(
    level: ConeLevel,
    glows: &Query<&SceneryLight>,
    spots: &Query<(&SpotLight, &ScriptLight)>,
) -> f32 {
    match level {
        ConeLevel::Fixed(v) => v,
        ConeLevel::Glow(e) => glows.get(e).map_or(0.0, |l| l.level),
        ConeLevel::Spot(e) => spots.get(e).map_or(0.0, |(spot, script)| {
            if script.base_intensity > 0.0 {
                (spot.intensity / script.base_intensity).clamp(0.0, 1.0)
            } else {
                0.0
            }
        }),
    }
}

/// Sets visibility and tag of the active cones for the current light levels; returns how many are
/// lit.
pub(super) fn apply_levels(p: &mut LightConePoolParams, gate: f32) -> usize {
    let mut lit = 0;
    p.pool.active.retain(|&e| {
        let Ok((cone, mut vis, mut tag)) = p.apply.get_mut(e) else {
            return false;
        };
        let raw = if cone.kind.allowed(&p.quality) {
            level_of(cone.level, &p.glows, &p.spots)
        } else {
            0.0
        };
        if raw > CONE_ON_LEVEL {
            lit += 1;
        }
        let level = raw * gate;
        let on = level > CONE_ON_LEVEL;
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if on {
            let packed = pack_tag(
                cone.shape.color,
                level,
                cone.shape.inner_frac,
                cone.shape.ground_h,
            );
            if tag.value != packed {
                tag.value = packed;
            }
        }
        true
    });
    lit
}
