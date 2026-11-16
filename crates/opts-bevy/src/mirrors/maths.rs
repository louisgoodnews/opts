//! Pure geometry and selection of the mirrors: reflection, pose, field of view, pane and UV.

use super::BusMirrorCamera;
use bevy::prelude::*;

use super::settings::{MirrorAdjust, MIRROR_FOV_RANGE_DEG, MIRROR_OFFSET_M, MIRROR_VIEW_COS};

// ---------------------------------------------------------------------------------------------
// Pure maths
// ---------------------------------------------------------------------------------------------

/// The direction `d` mirrored at the plane with unit normal `n`.
#[must_use]
pub fn reflect(d: Vec3, n: Vec3) -> Vec3 {
    d - 2.0 * d.dot(n) * n
}

/// `n` or `-n`, whichever faces the viewer who looks along `view` (so `view . result <= 0`).
#[must_use]
pub fn facing_normal(n: Vec3, view: Vec3) -> Vec3 {
    if view.dot(n) > 0.0 {
        -n
    } else {
        n
    }
}

/// Camera position and view direction for the pane at `centre` with (unit) `normal`, seen from
/// `viewer`: the camera sits [`MIRROR_OFFSET_M`] in front of the pane and looks along the
/// viewer's ray reflected at the plane, which is what the viewer sees in the mirror. `None` when
/// the viewer is at the pane or the normal is degenerate.
#[must_use]
pub fn mirror_camera_pose(viewer: Vec3, centre: Vec3, normal: Vec3) -> Option<(Vec3, Vec3)> {
    let view = (centre - viewer).try_normalize()?;
    let n = normal.try_normalize()?;
    let n = facing_normal(n, view);
    Some((centre + n * MIRROR_OFFSET_M, reflect(view, n)))
}

/// Pose of a `.bus` mirror camera in the vehicle's frame (Bevy axes) and its field of view in
/// radians. The pictures are square (the `reflexion<N>.bmp` of the install are), so the
/// horizontal field of view of the tag is the vertical one too. `adjust` is the user's offset.
#[must_use]
pub fn tag_pose(tag: &BusMirrorCamera, adjust: &MirrorAdjust) -> (Vec3, Quat, f32) {
    let position = Vec3::new(tag.x, tag.z, -tag.y);
    // The mirror tags' yaw grows counter-clockwise seen from above, like Bevy's (the opposite of
    // the driver cameras'): only then do the side mirrors of the stock vehicles look outwards,
    // as the reflection of the driver's view at the pane does (see `bus.md`).
    let rotation = Quat::from_euler(
        EulerRot::YXZ,
        (tag.yaw_deg + adjust.yaw_deg).to_radians(),
        (tag.pitch_deg + adjust.pitch_deg).to_radians(),
        0.0,
    );
    let fov = (tag.fov_deg + adjust.fov_deg)
        .clamp(10.0, 160.0)
        .to_radians();
    (position, rotation, fov)
}

/// The vertical field of view (radians) under which a pane of `height` metres is seen from
/// `distance` metres, clamped to [`MIRROR_FOV_RANGE_DEG`].
#[must_use]
pub fn mirror_fov(height: f32, distance: f32) -> f32 {
    let (lo, hi) = MIRROR_FOV_RANGE_DEG;
    let fov = 2.0 * (height.max(0.01) / (2.0 * distance.max(0.2))).atan();
    fov.clamp(lo.to_radians(), hi.to_radians())
}

/// Whether the pane at `mirror` is in front of a camera at `cam` looking along `forward` (unit).
#[must_use]
pub fn mirror_in_front(cam: Vec3, forward: Vec3, mirror: Vec3) -> bool {
    (mirror - cam)
        .try_normalize()
        .is_some_and(|d| d.dot(forward) > MIRROR_VIEW_COS)
}

/// Whether the mirror of selection rank `rank` renders in frame `frame` when it updates every
/// `every`-th frame (the ranks are staggered).
#[must_use]
pub fn is_render_frame(frame: u64, rank: usize, every: u8) -> bool {
    let every = u64::from(every.max(1));
    (frame + rank as u64).is_multiple_of(every)
}

/// The indices of the `count` smallest distances, nearest first (ties by index). Non-finite
/// distances (a group without a visible pane) are never picked.
#[must_use]
pub fn nearest(distances: &[f32], count: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..distances.len())
        .filter(|i| distances[*i].is_finite())
        .collect();
    order.sort_by(|a, b| distances[*a].total_cmp(&distances[*b]).then(a.cmp(b)));
    order.truncate(count);
    order
}

/// Centre, mean normal, height and aspect of a pane mesh (local space): the height is the extent
/// along the local up direction projected onto the pane's plane (the width along the other
/// in-plane axis; a pane facing up or down falls back to the local X axis).
#[must_use]
pub fn pane_geometry(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
) -> Option<(Vec3, Vec3, f32, f32)> {
    if positions.is_empty() {
        return None;
    }
    let normal = normals
        .iter()
        .fold(Vec3::ZERO, |acc, n| acc + Vec3::from(*n))
        .try_normalize()?;
    let up = (Vec3::Y - normal * normal.dot(Vec3::Y))
        .try_normalize()
        .filter(|u| u.length() > 0.5 && normal.dot(Vec3::Y).abs() < 0.97)
        .or_else(|| (Vec3::X - normal * normal.dot(Vec3::X)).try_normalize())?;
    let right = up.cross(normal);
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    let (mut ulo, mut uhi, mut rlo, mut rhi) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for p in positions {
        let p = Vec3::from(*p);
        lo = lo.min(p);
        hi = hi.max(p);
        let (u, r) = (p.dot(up), p.dot(right));
        ulo = ulo.min(u);
        uhi = uhi.max(u);
        rlo = rlo.min(r);
        rhi = rhi.max(r);
    }
    let (height, width) = ((uhi - ulo).max(1e-4), (rhi - rlo).max(1e-4));
    Some(((lo + hi) * 0.5, normal, height, width / height))
}

/// The UV rectangle of a mesh.
#[must_use]
pub fn uv_bounds(uvs: &[[f32; 2]]) -> Option<(Vec2, Vec2)> {
    if uvs.is_empty() {
        return None;
    }
    let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for uv in uvs {
        lo = lo.min(Vec2::from(*uv));
        hi = hi.max(Vec2::from(*uv));
    }
    Some((lo, hi))
}

/// The UV transform that stretches the rectangle `[min, max]` of the pane's UVs over the whole
/// picture (the pane mesh only covers a part of its texture), optionally mirrored.
#[must_use]
pub fn pane_uv_transform(min: Vec2, max: Vec2, flip_u: bool, flip_v: bool) -> bevy::math::Affine2 {
    let size = (max - min).max(Vec2::splat(1e-4));
    let (sx, tx) = if flip_u {
        (-1.0 / size.x, max.x / size.x)
    } else {
        (1.0 / size.x, -min.x / size.x)
    };
    let (sy, ty) = if flip_v {
        (-1.0 / size.y, max.y / size.y)
    } else {
        (1.0 / size.y, -min.y / size.y)
    };
    bevy::math::Affine2::from_cols(Vec2::new(sx, 0.0), Vec2::new(0.0, sy), Vec2::new(tx, ty))
}
