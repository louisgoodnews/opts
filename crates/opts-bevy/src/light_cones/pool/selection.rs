//! Choosing the cones that shine: the screen coverage of a cone and the nearest candidates within
//! the budget, with hysteresis.

use super::*;

/// Screen coverage of a cone of `length` and base `radius` whose centre is `distance` away: its
/// projected area up to a constant, `length * radius / distance^2` (distances below 1 m count as
/// 1 m).
#[must_use]
pub fn cone_coverage(length: f32, radius: f32, distance: f32) -> f32 {
    length * radius / (distance * distance).max(1.0)
}

/// The cones that should be visible: the `cap` of `candidates` (cone, coverage) with the
/// largest coverage, where a cone in `current` counts `hysteresis` times its coverage. Ties
/// break by entity, so the choice is stable from one selection to the next.
#[must_use]
pub fn select_cones(
    candidates: &[(Entity, f32)],
    current: &[Entity],
    cap: usize,
    hysteresis: f32,
) -> Vec<Entity> {
    if cap == 0 {
        return Vec::new();
    }
    let mut ranked: Vec<(f32, Entity)> = candidates
        .iter()
        .map(|&(e, s)| {
            (
                if current.contains(&e) {
                    s * hysteresis
                } else {
                    s
                },
                e,
            )
        })
        .collect();
    let better = |a: &(f32, Entity), b: &(f32, Entity)| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1));
    if ranked.len() > cap {
        ranked.select_nth_unstable_by(cap - 1, better);
        ranked.truncate(cap);
    }
    ranked.sort_by(better);
    ranked.into_iter().map(|(_, e)| e).collect()
}

pub(super) fn test_cone_candidate(
    cone: &LightCone,
    global: &GlobalTransform,
    cam: Vec3,
    fwd: Vec3,
    quality: &LightQuality,
    glows: &Query<&SceneryLight>,
    spots: &Query<(&SpotLight, &ScriptLight)>,
) -> Option<f32> {
    if !cone.kind.allowed(quality) || level_of(cone.level, glows, spots) <= CONE_ON_LEVEL {
        return None;
    }
    let m = global.affine().matrix3;
    let axis = -Vec3::from(m.z_axis);
    let length = axis.length();
    let radius = Vec3::from(m.x_axis).length();
    let apex = global.translation();
    let centre = apex + axis * 0.5;
    let bound = (length * 0.5).max(radius);
    let to_centre = centre - cam;
    if to_centre.length() - bound > cone.kind.max_distance_m() || to_centre.dot(fwd) < -bound {
        return None;
    }
    Some(cone_coverage(length, radius, to_centre.length()))
}
