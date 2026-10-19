//! The shape of a cone: kind, angles and length for lanterns, headlights and spotlights.

use super::*;

/// Which setting gates a cone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConeKind {
    /// A scenery lantern (`LightQuality::lantern_cones`).
    Lantern,
    /// A vehicle headlight (`LightQuality::headlight_cones`).
    Headlight,
}

impl ConeKind {
    /// Whether `quality` allows this kind of cone at all.
    #[must_use]
    pub const fn allowed(self, quality: &LightQuality) -> bool {
        match self {
            Self::Lantern => quality.lantern_cones,
            Self::Headlight => quality.headlight_cones,
        }
    }

    /// Cones farther than this from the camera are not considered (m).
    #[must_use]
    pub const fn max_distance_m(self) -> f32 {
        match self {
            Self::Lantern => LANTERN_MAX_DISTANCE_M,
            Self::Headlight => HEADLIGHT_MAX_DISTANCE_M,
        }
    }
}

/// Static geometry and colour of a cone, in its parent's frame (Bevy axes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConeShape {
    /// The apex (the lamp).
    pub apex: Vec3,
    /// Unit direction the cone opens to.
    pub dir: Vec3,
    /// Length along the axis (m).
    pub length: f32,
    /// Half angle of the outer cone (radians).
    pub half_angle: f32,
    /// Radius, as a fraction of the outer radius, where the soft edge starts (`0..=0.95`).
    pub inner_frac: f32,
    /// Colour, `0..=1` per channel.
    pub color: [f32; 3],
    /// Height of the apex above the ground it stands on (m), if known: the cone ends at that
    /// horizontal plane (a soft base) and never extends below it.
    pub ground_h: Option<f32>,
}

impl ConeShape {
    /// Radius of the base (m).
    #[must_use]
    pub fn base_radius(&self) -> f32 {
        self.length * self.half_angle.tan()
    }

    /// The transform that maps the unit cone onto this shape: apex at the translation, axis -Z
    /// turned to `dir`, X and Y scaled to the base radius, Z to the length.
    #[must_use]
    pub fn transform(&self) -> Transform {
        let r = self.base_radius();
        Transform {
            translation: self.apex,
            rotation: Quat::from_rotation_arc(Vec3::NEG_Z, self.dir),
            scale: Vec3::new(r, r, self.length),
        }
    }
}

/// Half angle (radians) and inner-edge fraction from a light's full inner and outer angles
/// (degrees): the half angle is `outer / 2` kept in `half_deg`, the inner half angle keeps the
/// ratio `inner / outer`.
#[must_use]
pub fn cone_angles(inner_deg: f32, outer_deg: f32, half_deg: (f32, f32)) -> (f32, f32) {
    let outer = if outer_deg > 0.0 {
        outer_deg
    } else {
        half_deg.1 * 2.0
    };
    let half = (outer * 0.5).clamp(half_deg.0, half_deg.1);
    let ratio = (inner_deg / outer).clamp(0.0, 0.95);
    let half_rad = half.to_radians();
    let inner_rad = half_rad * ratio;
    (
        half_rad,
        (inner_rad.tan() / half_rad.tan()).clamp(0.0, 0.95),
    )
}

fn color01(rgb: [f32; 3]) -> [f32; 3] {
    rgb.map(|c| (c / 255.0).clamp(0.0, 1.0))
}

/// Length of a lantern cone: the way from the apex to the ground when it points down (the
/// object's origin is at ground level), else the default.
#[must_use]
pub fn lantern_cone_length(apex: Vec3, dir: Vec3) -> f32 {
    if dir.y < -0.2 && apex.y > 0.0 {
        (apex.y / -dir.y).clamp(LANTERN_LENGTH_M.0, LANTERN_LENGTH_M.1)
    } else {
        LANTERN_DEFAULT_LENGTH_M
    }
}

/// The cone of a scenery `[light_enh_2]`, in the object's frame. A light has one when it is
/// directional and carries the `cone` flag (parameter 22); omnidirectional lights, traffic
/// signals and the like do not.
#[must_use]
pub fn lantern_cone_shape(light: &ScoLight) -> Option<ConeShape> {
    if light.omni || !light.volumetric_cone {
        return None;
    }
    let dir = omsi_to_bevy_pos(light.direction).normalize_or_zero();
    if dir == Vec3::ZERO {
        return None;
    }
    let apex = omsi_to_bevy_pos(light.position);
    let (half_angle, inner_frac) =
        cone_angles(light.cone_inner_deg, light.cone_outer_deg, LANTERN_HALF_DEG);
    Some(ConeShape {
        apex,
        dir,
        length: lantern_cone_length(apex, dir),
        half_angle,
        inner_frac,
        color: color01(light.color),
        ground_h: ground_height(apex.y),
    })
}

/// The cone of a vehicle `[light_enh_2]` headlight (the caller checked
/// [`is_headlight`](crate::vehicle_glow::is_headlight)), in the vehicle unit's frame. The
/// light has no range: the cone is [`HEADLIGHT_CONE_LENGTH_M`] long.
#[must_use]
pub fn headlight_cone_shape(light: &ScoLight) -> Option<ConeShape> {
    let dir = omsi_to_bevy_pos(light.direction).normalize_or_zero();
    if dir == Vec3::ZERO {
        return None;
    }
    let (half_angle, inner_frac) = cone_angles(
        light.cone_inner_deg,
        light.cone_outer_deg,
        HEADLIGHT_HALF_DEG,
    );
    let apex = omsi_to_bevy_pos(light.position);
    Some(ConeShape {
        apex,
        dir,
        length: HEADLIGHT_CONE_LENGTH_M,
        half_angle,
        inner_frac,
        color: color01(light.color),
        ground_h: ground_height(apex.y),
    })
}

/// The cone of a `[spotlight]`, in the frame of the Bevy `SpotLight` entity (apex at its origin,
/// shining along -Z, as `spawn_vehicle_model` orients it). The angles are full angles like
/// those of `[light_enh_2]` (Direct3D spotlight convention) and the range is capped at
/// [`SPOT_CONE_MAX_LENGTH_M`].
#[must_use]
pub fn spotlight_cone_shape(spot: &Spotlight) -> ConeShape {
    let (half_angle, inner_frac) =
        cone_angles(spot.inner_cone, spot.outer_cone, HEADLIGHT_HALF_DEG);
    let length = if spot.range > 0.0 {
        spot.range.min(SPOT_CONE_MAX_LENGTH_M)
    } else {
        SPOT_CONE_MAX_LENGTH_M
    };
    ConeShape {
        apex: Vec3::ZERO,
        dir: Vec3::NEG_Z,
        length,
        half_angle,
        inner_frac,
        color: color01(spot.color_rgb.map(f32::from)),
        ground_h: ground_height(omsi_to_bevy_pos(spot.pos).y),
    }
}
