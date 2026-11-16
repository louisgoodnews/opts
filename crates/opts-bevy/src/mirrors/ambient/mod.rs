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

mod cameras;
mod candidates;
mod panes;
mod pool;

pub use cameras::AmbientCamera;
pub use panes::{register_ambient_panes, AmbientPane};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::mirrors::*;

use cameras::{assign_free_cameras, pose_ambient_cameras, AmbientCameraItem, MainCameraAmbientFilter};
use candidates::collect_candidates;
use panes::AmbientPaneItem;
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
