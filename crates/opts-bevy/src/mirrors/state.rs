//! Per-group state of the mirrors: the frame counter, the statistics, groups and cameras.

use super::BusMirrorCamera;
use bevy::pbr::StandardMaterial;
use bevy::prelude::*;

use crate::glass::mirror_index;

/// Counts update frames (the turn of a mirror is derived from it).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct MirrorFrame(pub u64);

/// What the mirror system did in the last frame (`sim.get` shows it).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MirrorStats {
    /// Mirror cameras (groups) that exist.
    pub cameras: usize,
    /// Groups whose panes are in the player's view (or that stream into a panel): they show a
    /// live picture and share the render budget.
    pub selected: usize,
    /// Mirror cameras active (rendering) in the last frame.
    pub rendering: usize,
    /// Mirrors of other vehicles (parked, AI) that have a live picture.
    pub ambient: usize,
}

/// What panes share a camera: the panes of one `reflexion<N>.bmp` number (the main and the
/// wide-angle pane of a side mirror have different pooled materials but one `.bus` camera), or,
/// for a pane whose number is unknown, the panes of one pooled material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupKey {
    /// Mirror number `N` of `reflexion<N>.bmp`.
    Index(u32),
    /// A pooled material without a number.
    Material(AssetId<StandardMaterial>),
}

impl GroupKey {
    /// The group of the pane material `id`.
    #[must_use]
    pub fn of(id: AssetId<StandardMaterial>) -> Self {
        mirror_index(id).map_or(Self::Material(id), Self::Index)
    }
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
    /// The pane's own (static, pooled) material, which comes back when the group is not live.
    pub original: Handle<StandardMaterial>,
}

/// A mirror camera (one per group of panes that share a material).
#[derive(Component, Debug, Clone)]
pub struct MirrorCamera {
    /// The player vehicle it belongs to.
    pub vehicle: Entity,
    /// What the panes of the group have in common.
    pub key: GroupKey,
    /// The static (pooled) material the live picture's material is made from.
    pub original: Handle<StandardMaterial>,
    /// The private copy that shows the camera's picture.
    pub dynamic: Handle<StandardMaterial>,
    /// The panes.
    pub members: Vec<MirrorMember>,
    /// Union of the members' UV rectangles.
    pub uv_min: Vec2,
    /// Union of the members' UV rectangles.
    pub uv_max: Vec2,
    /// The picture the camera renders into.
    pub picture: Handle<Image>,
    /// The picture's size when it does not stream.
    pub base_size: UVec2,
    /// The picture is at streaming size (virtual mirror).
    pub enlarged: bool,
    /// The `.bus` camera of this pane (`reflexion<N>.bmp` is tag N), when the vehicle has one and
    /// [`crate::mirrors::settings::MirrorSettings::use_tags`] is on: the camera then follows it instead of the reflection.
    pub tag: Option<BusMirrorCamera>,
    /// [`MirrorFrame`] of the camera's last render (0 = never): the oldest picture in view
    /// renders next ([`crate::mirrors::pick_renders`]).
    pub last_render: u64,
}

/// On a pane mesh that belongs to a group.
#[derive(Component, Debug, Clone, Copy)]
pub struct MirrorSurface {
    /// The group's camera.
    pub camera: Entity,
}
