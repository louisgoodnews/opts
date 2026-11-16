//! Centreline poses and cross-section stations of a placed spline, in the tile-local OMSI frame.

use super::MapSplineInstance;

/// Height above the segment start at arc length `s`.
///
/// `[spline_h]` records state the total height change explicitly (parameter 13); the profile is
/// then a cubic Hermite curve between the end gradients (a smooth ramp). Plain `[spline]` records
/// change the gradient linearly from start to end.
pub(super) fn height_offset(inst: &MapSplineInstance, s: f32, length: f32) -> f32 {
    debug_assert!((length - inst.length_m.max(0.01)).abs() < 1e-3);
    inst.height_offset(s)
}

/// Centreline pose of a placed spline at arc length `s`, in the spline's tile-local OMSI frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplinePose {
    /// Easting (m, tile-local).
    pub x: f32,
    /// Northing (m, tile-local).
    pub y: f32,
    /// Elevation (m, absolute).
    pub z: f32,
    /// Heading in radians, clockwise from north.
    pub heading: f32,
}

/// Pose on the centreline of `inst` at arc length `s` (clamped to the segment).
#[must_use]
pub fn spline_pose(inst: &MapSplineInstance, s: f32) -> SplinePose {
    let length = inst.length_m.max(0.01);
    let s = s.clamp(0.0, length);
    let phi = inst.heading_deg.to_radians();
    let (heading, x, y) = if inst.radius_m.abs() > 1e-3 {
        let r = inst.radius_m;
        let h = phi + s / r;
        (
            h,
            inst.position.x - r * (h.cos() - phi.cos()),
            inst.position.y + r * (h.sin() - phi.sin()),
        )
    } else {
        (
            phi,
            inst.position.x + s * phi.sin(),
            inst.position.y + s * phi.cos(),
        )
    };
    SplinePose {
        x,
        y,
        z: inst.position.z + height_offset(inst, s, length),
        heading,
    }
}

/// The cross-sections a placed spline is drawn at: `(east, north, elevation, heading, s)` in the
/// frame whose origin is `tile_origin` (the spline's tile corner in that frame). Straight
/// segments with a constant slope need only their two ends; arcs and changing gradients
/// (including `[spline_h]` ramps) are sampled every 4 m or 5°. The terrain carve builds its
/// ribbons on the same stations so the carved ground matches the drawn surface.
#[must_use]
pub fn spline_stations(
    inst: &MapSplineInstance,
    tile_origin: (f32, f32),
) -> Vec<(f32, f32, f32, f32, f32)> {
    let length = inst.length_m.max(0.01);
    let arc = inst.radius_m.abs() > 1e-3;
    let by_len = (length / 4.0).ceil();
    let by_angle = if arc {
        (length / inst.radius_m.abs()).to_degrees().abs() / 5.0
    } else {
        0.0
    };
    // A straight segment with constant slope is exactly representable by one quad per ribbon;
    // a `[spline_h]` ramp is a cubic even between equal end gradients.
    let ramp = [0.25, 0.5, 0.75].iter().any(|f| {
        (height_offset(inst, length * f, length) - inst.gradient_start * length * f).abs() > 1e-4
    });
    let steps = if !arc && !ramp && (inst.gradient_start - inst.gradient_end).abs() < 1e-6 {
        1
    } else {
        by_len.max(by_angle.ceil()).clamp(1.0, 400.0) as usize
    };

    let phi = inst.heading_deg.to_radians();
    let p0 = (
        tile_origin.0 + inst.position.x,
        tile_origin.1 + inst.position.y,
    );

    // Centreline samples: (east, north, elevation, heading).
    let mut samples = Vec::with_capacity(steps + 1);
    for k in 0..=steps {
        let s = length * k as f32 / steps as f32;
        let (h, ex, ny) = if arc {
            let r = inst.radius_m;
            let h = phi + s / r;
            (
                h,
                p0.0 - r * (h.cos() - phi.cos()),
                p0.1 + r * (h.sin() - phi.sin()),
            )
        } else {
            (phi, p0.0 + s * phi.sin(), p0.1 + s * phi.cos())
        };
        let z = inst.position.z + height_offset(inst, s, length);
        samples.push((ex, ny, z, h, s));
    }

    samples
}
