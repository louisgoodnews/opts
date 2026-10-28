//! Which mirror renders when: every mirror pane in view shows a live picture, and the budget
//! ([`LightQuality::mirror_count`] pictures every [`LightQuality::mirror_every`] frames) is shared
//! out round-robin among them, oldest picture first.
//!
//! Before 2026-10-26 the budget chose the `mirror_count` mirrors *nearest to the viewer*, whether
//! or not they were in view, and every other pane fell back to its static `reflexion<N>.bmp`. A
//! bus with more than two mirrors (the Valiant 7700A has a left, a right, a right wide-angle and
//! an interior mirror) therefore always showed the stock road scene on the far mirrors, even while
//! the driver looked straight at them. Now the same number of renders per frame is spent on the
//! mirrors that are actually visible; with more mirrors in view than the budget they take turns,
//! so each one updates a little less often instead of not at all.

use super::*;

/// Smallest angle (radians) a pane must subtend from the viewer to get a live picture: below it the
/// pane covers a few pixels and a render is wasted (about 6 px at a 40 degree, 1000 px high view).
pub const MIRROR_MIN_ANGLE: f32 = 0.005;

/// Whether a pane of size `radius` at `point` may be visible to a perspective camera at `cam`
/// (world transform) with vertical field of view `fov` (radians) and `aspect` (width / height).
/// Conservative: the pane counts as soon as its bounding sphere touches the view pyramid.
#[must_use]
pub fn mirror_in_view(
    cam: &GlobalTransform,
    fov: f32,
    aspect: f32,
    point: Vec3,
    radius: f32,
) -> bool {
    let local = cam.affine().inverse().transform_point3(point);
    let depth = -local.z;
    if depth <= -radius {
        return false;
    }
    let tv = (fov.clamp(0.01, 3.1) * 0.5).tan();
    let th = tv * aspect.max(0.01);
    // A sphere's distance to a side plane of the pyramid is at most `radius / cos`; scale the
    // margin generously so a pane at the screen edge is never missed.
    let margin_v = radius * (1.0 + tv * tv).sqrt();
    let margin_h = radius * (1.0 + th * th).sqrt();
    let depth = depth.max(0.0);
    local.x.abs() <= th * depth + margin_h && local.y.abs() <= tv * depth + margin_v
}

/// How many mirror pictures render in `frame`: `count` pictures every `every` frames, spread over
/// the frames (the ranks of [`is_render_frame`]), so a budget of 2 every 2nd frame is one picture
/// per frame.
#[must_use]
pub fn renders_in_frame(frame: u64, count: u8, every: u8) -> usize {
    (0..usize::from(count))
        .filter(|k| is_render_frame(frame, *k, every))
        .count()
}

/// The `n` mirrors that render now, as indices into the slices: only `eligible` ones, the one
/// whose picture is oldest (`last` = frame of its last render, 0 for never) first, ties to the
/// nearest (`distance`), then by index.
#[must_use]
pub fn pick_renders(last: &[u64], distance: &[f32], eligible: &[bool], n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..last.len()).filter(|i| eligible[*i]).collect();
    order.sort_by(|a, b| {
        last[*a]
            .cmp(&last[*b])
            .then(distance[*a].total_cmp(&distance[*b]))
            .then(a.cmp(b))
    });
    order.truncate(n);
    order
}
