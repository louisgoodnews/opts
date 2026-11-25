//! Picking against the trace: a ray (camera through a pixel) against the oriented bounding boxes
//! of the records.

use bevy::math::{Quat, Vec3};

use super::TraceRecord;

/// One record hit by a ray.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// Id of the record.
    pub id: u64,
    /// Distance from the ray origin to the entry point (m).
    pub distance: f32,
}

/// Distance at which the ray enters the record's box, if it does.
#[must_use]
pub fn ray_hits(rec: &TraceRecord, origin: Vec3, dir: Vec3) -> Option<f32> {
    let (w, b) = (rec.world?, rec.bounds?);
    let rot = Quat::from_xyzw(w.q[0], w.q[1], w.q[2], w.q[3]);
    let scale = Vec3::from(w.s);
    let inv = rot.inverse();
    // Ray into local space (scale undone after the rotation).
    let o = (inv * (origin - Vec3::from(w.t))) / scale;
    let d = (inv * dir) / scale;
    let (min, max) = (Vec3::from(b.min), Vec3::from(b.max));
    let mut t0 = 0.0_f32;
    let mut t1 = f32::MAX;
    for axis in 0..3 {
        let (oa, da) = (o[axis], d[axis]);
        if da.abs() < 1e-9 {
            if oa < min[axis] || oa > max[axis] {
                return None;
            }
            continue;
        }
        let (mut a, mut c) = ((min[axis] - oa) / da, (max[axis] - oa) / da);
        if a > c {
            std::mem::swap(&mut a, &mut c);
        }
        t0 = t0.max(a);
        t1 = t1.min(c);
        if t0 > t1 {
            return None;
        }
    }
    // `d` was divided by the scale, so the parameter is still in world metres along `dir`.
    Some(t0)
}

/// All records the ray hits, nearest first. Records without a box (terrain, splines without
/// geometry) are not pickable here.
#[must_use]
pub fn pick<'a>(
    records: impl IntoIterator<Item = &'a TraceRecord>,
    origin: Vec3,
    dir: Vec3,
) -> Vec<Hit> {
    let dir = dir.normalize_or_zero();
    if dir == Vec3::ZERO {
        return Vec::new();
    }
    let mut hits: Vec<Hit> = records
        .into_iter()
        .filter_map(|r| ray_hits(r, origin, dir).map(|distance| Hit { id: r.id, distance }))
        .collect();
    hits.sort_by(|a, b| a.distance.total_cmp(&b.distance));
    hits
}
