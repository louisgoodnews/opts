//! Live mirrors on the vehicles around the player (parked cars, AI traffic).
//!
//! Only the player's own vehicle gets a camera per mirror ([`super::spawn`]). Every other
//! vehicle would keep the static `reflexion<N>.bmp` of its pane, a fixed road scene. This module
//! gives the **nearest** other mirrors a live picture, with the geometric reflection of the
//! viewer's ray ([`mirror_camera_pose`]):
//!
//! * **Panes.** [`register_ambient_panes`] marks every mesh that gets a mirror material
//!   ([`SurfaceKind::Mirror`]) with an [`AmbientPane`] (its geometry, UV rectangle and own static
//!   material), once, when the mesh appears. The player's own panes carry the mark too and are
//!   skipped when choosing (they have [`MirrorSurface`] or a [`PlayerVehicle`] ancestor).
//! * **Budget.** [`LightQuality::mirror_count`] cameras (the same budget as the player's mirrors:
//!   2 up to level 3, 1 above) serve the panes that are visible, within
//!   [`AMBIENT_RANGE_M`], in front of the main camera and facing it, nearest first. A pane keeps
//!   its camera while it stays among the nearest, so the picture does not jump around.
//! * **Cost.** Small pictures ([`AMBIENT_PICTURE`] pixels square) rendered every
//!   [`AMBIENT_EVERY_FACTOR`] times as seldom as the player's mirrors, staggered by rank. A pane
//!   that loses its camera gets its own static material back.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::spawn::mirror_material;
use crate::mirrors::*;

mod candidates;
mod pool;

use candidates::{collect_candidates, Candidate};
use pool::{restore_pane_material, sync_pool_capacity};

/// Farthest distance (m) at which another vehicle's mirror is live.
pub const AMBIENT_RANGE_M: f32 = 30.0;
/// Side of the picture (pixels).
pub const AMBIENT_PICTURE: u32 = 192;
/// How many times less often than the player's mirrors an ambient mirror renders.
pub const AMBIENT_EVERY_FACTOR: u8 = 2;
/// Smallest `|cos|` between the pane's normal and the direction to the viewer: a pane seen
/// edge-on is not worth a camera.
pub const AMBIENT_MIN_FACING: f32 = 0.15;

/// A mirror pane of any vehicle (set once, when the mesh appears).
#[derive(Component, Debug, Clone)]
pub struct AmbientPane {
    /// Centre of the pane in the mesh's local space.
    pub centre: Vec3,
    /// Mean normal of the pane in the mesh's local space.
    pub normal: Vec3,
    /// Extent along the pane's up direction (local metres, before scale).
    pub height: f32,
    /// Width over height.
    pub aspect: f32,
    /// The pane's UV rectangle.
    pub uv_min: Vec2,
    /// The pane's UV rectangle.
    pub uv_max: Vec2,
    /// The pane's own static material.
    pub original: Handle<StandardMaterial>,
}

/// A camera of the pool, and the pane it serves.
#[derive(Component, Debug, Clone)]
pub struct AmbientCamera {
    /// The picture it renders into.
    pub picture: Handle<Image>,
    /// The material that shows the picture on the pane.
    pub dynamic: Handle<StandardMaterial>,
    /// The pane it serves now.
    pub pane: Option<Entity>,
}

type NewAmbientPaneItem<'a> = (Entity, &'a MeshMaterial3d<StandardMaterial>, &'a Mesh3d);
type NewAmbientPaneFilter = (
    Added<MeshMaterial3d<StandardMaterial>>,
    Without<AmbientPane>,
);

/// Marks new mirror meshes.
pub fn register_ambient_panes(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    new: Query<NewAmbientPaneItem, NewAmbientPaneFilter>,
) {
    for (entity, material, mesh) in &new {
        if surface_kind(material.0.id()) != Some(SurfaceKind::Mirror) {
            continue;
        }
        let Some(mesh) = meshes.get(&mesh.0) else {
            continue;
        };
        let (Some(pos), Some(nor)) = (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                .and_then(|a| a.as_float3()),
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
                .and_then(|a| a.as_float3()),
        ) else {
            continue;
        };
        let Some((centre, normal, height, aspect)) = pane_geometry(pos, nor) else {
            continue;
        };
        let (uv_min, uv_max) = mesh
            .attribute(Mesh::ATTRIBUTE_UV_0)
            .and_then(|a| match a {
                bevy::mesh::VertexAttributeValues::Float32x2(v) => uv_bounds(v),
                _ => None,
            })
            .unwrap_or((Vec2::ZERO, Vec2::ONE));
        commands.entity(entity).insert(AmbientPane {
            centre,
            normal,
            height,
            aspect,
            uv_min,
            uv_max,
            original: material.0.clone(),
        });
    }
}

type AmbientPaneItem<'a> = (
    Entity,
    &'a AmbientPane,
    &'a GlobalTransform,
    Option<&'a InheritedVisibility>,
);

type AmbientCameraItem<'a> = (
    Entity,
    &'a mut AmbientCamera,
    &'a mut Transform,
    &'a mut Camera,
    &'a mut Projection,
);

type MainCameraAmbientFilter = (
    With<MainCamera>,
    Without<AmbientCamera>,
    Without<MirrorCamera>,
);

/// Parameters for [`update_ambient_mirrors`].
#[derive(SystemParam)]
pub struct AmbientUpdateParams<'w, 's> {
    pub quality: Res<'w, LightQuality>,
    pub settings: Res<'w, MirrorSettings>,
    pub frame: Res<'w, MirrorFrame>,
    pub stats: ResMut<'w, MirrorStats>,
    pub images: ResMut<'w, Assets<Image>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub main: Query<'w, 's, &'static GlobalTransform, MainCameraAmbientFilter>,
    pub players: Query<'w, 's, (), With<PlayerVehicle>>,
    pub parents: Query<'w, 's, &'static ChildOf>,
    pub panes: Query<'w, 's, AmbientPaneItem<'static>, Without<MirrorSurface>>,
    pub pane_materials: Query<'w, 's, &'static mut MeshMaterial3d<StandardMaterial>>,
    pub cameras: Query<'w, 's, AmbientCameraItem<'static>>,
}

fn assign_free_cameras(
    free: Vec<Entity>,
    waiting: impl Iterator<Item = Entity>,
    cameras: &mut Query<AmbientCameraItem>,
    panes: &Query<AmbientPaneItem, Without<MirrorSurface>>,
    materials: &mut Assets<StandardMaterial>,
    settings: &MirrorSettings,
) {
    let mut waiting = waiting;
    for camera in free {
        let Some(pane_entity) = waiting.next() else {
            break;
        };
        let Ok((_, mut info, ..)) = cameras.get_mut(camera) else {
            continue;
        };
        if let Ok((_, pane, ..)) = panes.get(pane_entity) {
            if let Some(base) = materials.get(&pane.original).cloned() {
                let new = mirror_material(
                    &base,
                    info.picture.clone(),
                    pane.uv_min,
                    pane.uv_max,
                    settings,
                );
                if let Some(mut m) = materials.get_mut(&info.dynamic) {
                    *m = new;
                }
            }
        }
        info.pane = Some(pane_entity);
    }
}

fn pose_ambient_cameras(
    cameras: &mut Query<AmbientCameraItem>,
    candidates: &[Candidate],
    viewer: Vec3,
    frame: u64,
    every: u8,
    pane_materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    stats: &mut MirrorStats,
) {
    let mut rank = 0;
    for (_, info, mut t, mut cam, mut projection) in cameras.iter_mut() {
        let Some(pane) = info.pane else {
            if cam.is_active {
                cam.is_active = false;
            }
            continue;
        };
        let Some(c) = candidates.iter().find(|c| c.pane == pane) else {
            continue;
        };
        if let Ok(mut m) = pane_materials.get_mut(pane) {
            if m.0 != info.dynamic {
                m.0 = info.dynamic.clone();
            }
        }
        if let Some((position, direction)) = mirror_camera_pose(viewer, c.centre, c.normal) {
            t.translation = position;
            t.look_to(direction, Vec3::Y);
        }
        if let Projection::Perspective(persp) = &mut *projection {
            persp.fov = mirror_fov(c.height, c.distance);
            persp.aspect_ratio = c.aspect.clamp(MIRROR_ASPECT_RANGE.0, MIRROR_ASPECT_RANGE.1);
        }
        let render = is_render_frame(frame, rank, every);
        if cam.is_active != render {
            cam.is_active = render;
        }
        stats.ambient += 1;
        rank += 1;
    }
}

/// Chooses the panes, keeps the pool at the budget and poses the cameras.
pub fn update_ambient_mirrors(mut params: AmbientUpdateParams, mut commands: Commands) {
    let budget = usize::from(params.quality.mirror_count);
    sync_pool_capacity(
        budget,
        &params.cameras,
        &params.panes,
        &mut params.pane_materials,
        &mut params.images,
        &mut params.materials,
        &mut commands,
    );

    params.stats.ambient = 0;
    let Some(view) = params.main.iter().next() else {
        for (_, _, _, mut cam, _) in &mut params.cameras {
            if cam.is_active {
                cam.is_active = false;
            }
        }
        return;
    };
    if budget == 0 {
        return;
    }
    let (viewer, forward) = (view.translation(), view.forward().as_vec3());

    let candidates = collect_candidates(
        &params.panes,
        viewer,
        forward,
        &params.parents,
        &params.players,
        budget,
    );

    let mut taken: Vec<Entity> = Vec::new();
    let mut free: Vec<Entity> = Vec::new();
    for (entity, mut info, ..) in &mut params.cameras {
        match info.pane {
            Some(p) if candidates.iter().any(|c| c.pane == p) => taken.push(p),
            Some(p) => {
                restore_pane_material(p, &params.panes, &mut params.pane_materials);
                info.pane = None;
                free.push(entity);
            }
            None => free.push(entity),
        }
    }

    let waiting = candidates
        .iter()
        .map(|c| c.pane)
        .filter(|p| !taken.contains(p));
    assign_free_cameras(
        free,
        waiting,
        &mut params.cameras,
        &params.panes,
        &mut params.materials,
        &params.settings,
    );

    let every = params
        .quality
        .mirror_every
        .saturating_mul(AMBIENT_EVERY_FACTOR);
    pose_ambient_cameras(
        &mut params.cameras,
        &candidates,
        viewer,
        params.frame.0,
        every,
        &mut params.pane_materials,
        &mut params.stats,
    );
}
