//! Rear-view mirror cameras of the player's vehicle (`docs/src/architecture/reflections-research.md`
//! section 7 step A, `lighting-system.md` section 9, `camera-system.md`).
//!
//! * **Identification.** A mirror pane is a mesh of the [`PlayerVehicle`] whose material the model
//!   loader registered as [`SurfaceKind::Mirror`]: the texture `reflexion<N>.bmp`, OMSI's
//!   convention for the pane of mirror `N` (the housing of the same unit has another texture, so
//!   the pane is a separate part with only pane faces). AI and parked vehicles are never looked
//!   at. A scan every [`SCAN_INTERVAL_S`] real seconds finds new panes (the vehicle's parts spawn
//!   over several frames). Panes sharing one material form a **group** (the two slots of a side
//!   mirror, or the alternative mirror variants of a model, only one of which is visible); a group
//!   gets one camera, at most [`MAX_CAMERAS`] groups.
//! * **One camera per group**, a `Camera3d` rendering into a [`MirrorSettings::size`] image
//!   (8-bit; the camera carries `Hdr` like the main camera). The pose is the viewer-ray
//!   reflection of [`mirror_camera_pose`]: the camera sits just in front of the visible members'
//!   centre and looks along the main camera's view ray reflected at their mean normal. The
//!   vertical field of view is the angle the pane subtends from the viewer ([`mirror_fov`]); the
//!   projection aspect is the pane's own, so the picture is not distorted when it is stretched
//!   over the pane's UV rectangle ([`pane_uv_transform`]). The picture replaces the base-colour
//!   texture of a **private copy** of the material (the pooled material is shared with every other
//!   vehicle of the same type); the original comes back whenever the group is not selected,
//!   `mirror_count` is 0, or the vehicle is no longer the player's.
//! * **Budget.** [`LightQuality::mirror_count`] (0..=2) groups are selected, nearest to the
//!   viewer first; a selected group renders when it is in front of the main camera
//!   ([`mirror_in_front`]) and on its turn ([`is_render_frame`]: every `mirror_every`-th frame,
//!   staggered by rank so two mirrors do not render in the same frame). The switch is
//!   `Camera::is_active`, never the removal of `Camera3d`. Members that are hidden (a model keeps
//!   mutually exclusive variants) do not count for the pose and the distance.
//! * **Parity.** [`sync_mirror_camera_look`] copies `Hdr` and `DistanceFog` from the
//!   [`MainCamera`]; the mirror camera does **not** tonemap (the picture is scene-linear and is
//!   tonemapped once, by the main camera, when it is shown) and carries no colour grading for the
//!   same reason. Mirror cameras never carry [`MainCamera`], so every query that means the
//!   player's view skips them.

use std::collections::HashMap;

use bevy::camera::{Hdr, RenderTarget};
use bevy::pbr::{DistanceFog, StandardMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::Tonemapping;

use opts_core::formats::vehicles::BusMirrorCamera;

use crate::glass::{mirror_index, surface_kind, SurfaceKind};
use crate::light_quality::{LightQuality, LightQualitySet};
use crate::vehicle::BusVehicle;
use crate::vehicle_sim::PlayerVehicle;
use crate::MainCamera;

/// Most mirror groups (cameras) that exist; at most two render at a time.
pub const MAX_CAMERAS: usize = 6;
/// Real seconds between two scans of the player vehicle for mirror panes.
pub const SCAN_INTERVAL_S: f64 = 1.0;
/// Distance of the camera in front of the pane (m).
pub const MIRROR_OFFSET_M: f32 = 0.05;
/// Smallest and largest vertical field of view of a mirror picture (degrees).
pub const MIRROR_FOV_RANGE_DEG: (f32, f32) = (20.0, 70.0);
/// Range of the projection aspect (width / height) of a mirror picture.
pub const MIRROR_ASPECT_RANGE: (f32, f32) = (0.4, 3.0);
/// A pane counts as in front of the camera when the cosine of the angle between the view
/// direction and the direction to the pane is above this (about 78 degrees).
pub const MIRROR_VIEW_COS: f32 = 0.2;
/// Draw order of the mirror cameras: before the main camera.
pub const MIRROR_ORDER: isize = -10;

/// A user adjustment on top of the `.bus` mirror cameras (degrees), for a mirror whose tags do not
/// suit the user's seat or taste. `OPTS_MIRROR_ADJUST=yaw,pitch,fov` (any number of the three).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MirrorAdjust {
    /// Added to the tag's yaw (positive = further counter-clockwise seen from above).
    pub yaw_deg: f32,
    /// Added to the tag's pitch (positive = up).
    pub pitch_deg: f32,
    /// Added to the tag's field of view.
    pub fov_deg: f32,
}

/// Where the mirror pictures go and how they are laid on the pane.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct MirrorSettings {
    /// Texture size in pixels.
    pub size: UVec2,
    /// Mirror the picture left-right on the pane. Off by default: the panes of the inspected
    /// models carry UVs that already run right to left as the driver sees them (the texture is
    /// stored mirror-reversed), so the plain camera picture is the mirror image.
    pub flip_u: bool,
    /// Turn the picture upside down on the pane (a dev aid, off).
    pub flip_v: bool,
    /// Real seconds between two scans for new panes ([`SCAN_INTERVAL_S`]).
    pub scan_interval_s: f64,
    /// Drive the cameras from the vehicle's `[add_camera_reflexion]` tags (position, direction,
    /// field of view, square picture) when it has one for the pane; `false` keeps the geometric
    /// viewer-ray reflection for every pane. `OPTS_MIRROR_TAGS=0` switches it off.
    pub use_tags: bool,
    /// Adjustment of the tag cameras.
    pub adjust: MirrorAdjust,
}

impl Default for MirrorSettings {
    fn default() -> Self {
        Self {
            size: UVec2::new(256, 128),
            flip_u: false,
            flip_v: false,
            scan_interval_s: SCAN_INTERVAL_S,
            use_tags: true,
            adjust: MirrorAdjust::default(),
        }
    }
}

impl MirrorSettings {
    /// `OPTS_MIRROR_FLIP` (`u`, `v`, `uv` or `none`) and `OPTS_MIRROR_SIZE` (`256x128`) on top of
    /// the defaults: developer aids for checking the picture's orientation and cost.
    #[must_use]
    pub fn from_env() -> Self {
        let mut s = Self::default();
        if let Ok(f) = std::env::var("OPTS_MIRROR_FLIP") {
            s.flip_u = f.contains('u');
            s.flip_v = f.contains('v');
        }
        if let Ok(v) = std::env::var("OPTS_MIRROR_TAGS") {
            s.use_tags = !matches!(v.trim(), "0" | "off" | "false" | "no");
        }
        if let Ok(v) = std::env::var("OPTS_MIRROR_ADJUST") {
            let mut it = v.split(',').map(|n| n.trim().parse::<f32>().unwrap_or(0.0));
            s.adjust = MirrorAdjust {
                yaw_deg: it.next().unwrap_or(0.0),
                pitch_deg: it.next().unwrap_or(0.0),
                fov_deg: it.next().unwrap_or(0.0),
            };
        }
        if let Ok(v) = std::env::var("OPTS_MIRROR_SIZE") {
            if let Some((w, h)) = v.split_once('x') {
                if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                    s.size = UVec2::new(w.clamp(32, 1024), h.clamp(16, 1024));
                }
            }
        }
        s
    }
}

/// Counts update frames (the turn of a mirror is derived from it).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct MirrorFrame(pub u64);

/// What the mirror system did in the last frame (`sim.get` shows it).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MirrorStats {
    /// Mirror cameras (groups) that exist.
    pub cameras: usize,
    /// Groups selected for the quality (showing a live picture when visible).
    pub selected: usize,
    /// Mirror cameras active (rendering) in the last frame.
    pub rendering: usize,
}

/// One pane mesh of a mirror group.
#[derive(Debug, Clone)]
pub struct MirrorMember {
    /// The mesh entity.
    pub entity: Entity,
    /// Centre of the pane in the mesh's local space.
    pub centre: Vec3,
    /// Mean normal of the pane in the mesh's local space.
    pub normal: Vec3,
    /// Extent along the pane's up direction (local metres, before scale).
    pub height: f32,
    /// Width over height of the pane.
    pub aspect: f32,
}

/// A mirror camera (one per group of panes that share a material).
#[derive(Component, Debug, Clone)]
pub struct MirrorCamera {
    /// The player vehicle it belongs to.
    pub vehicle: Entity,
    /// The shared (pooled) material of the group.
    pub material: AssetId<StandardMaterial>,
    /// The static (original, pooled) material.
    pub original: Handle<StandardMaterial>,
    /// The private copy that shows the camera's picture.
    pub dynamic: Handle<StandardMaterial>,
    /// The panes.
    pub members: Vec<MirrorMember>,
    /// Union of the members' UV rectangles.
    pub uv_min: Vec2,
    /// Union of the members' UV rectangles.
    pub uv_max: Vec2,
    /// The `.bus` camera of this pane (`reflexion<N>.bmp` is tag N), when the vehicle has one and
    /// [`MirrorSettings::use_tags`] is on: the camera then follows it instead of the reflection.
    pub tag: Option<BusMirrorCamera>,
}

/// On a pane mesh that belongs to a group.
#[derive(Component, Debug, Clone, Copy)]
pub struct MirrorSurface {
    /// The group's camera.
    pub camera: Entity,
}

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

// ---------------------------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------------------------

mod spawn;
pub use spawn::spawn_mirror_cameras;

mod update;
pub use update::{sync_mirror_camera_look, update_mirror_cameras};

/// Registers the mirror cameras.
#[derive(Default)]
pub struct MirrorPlugin;

impl Plugin for MirrorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightQuality>()
            .insert_resource(MirrorSettings::from_env())
            .init_resource::<MirrorFrame>()
            .init_resource::<MirrorStats>()
            .add_systems(
                Update,
                spawn_mirror_cameras
                    .after(LightQualitySet)
                    .run_if(resource_exists::<Assets<Image>>)
                    .run_if(resource_exists::<Assets<StandardMaterial>>)
                    .run_if(resource_exists::<Assets<Mesh>>),
            )
            .add_systems(
                PostUpdate,
                (update_mirror_cameras, sync_mirror_camera_look)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}

#[cfg(test)]
mod tests;
