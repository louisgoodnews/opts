//! Per-frame mirror work: placing the cameras behind the mirrors and following the driver's look.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::schedule::{pick_renders, renders_in_frame};
use super::*;

mod cleanup;
mod pose;
mod render;

use cleanup::cleanup_dead_cameras;
use pose::{compute_group_poses, compute_in_view};
use render::{apply_camera_renders_and_poses, update_virtual_slots};

type MirrorCameraQueryItem<'a> = (
    Entity,
    &'a mut MirrorCamera,
    &'a mut Transform,
    &'a mut Camera,
    &'a mut Projection,
);

/// The main camera's place and projection.
type MainCameraView<'a> = (&'a GlobalTransform, Option<&'a Projection>);
/// The main camera, not a mirror camera.
type MainCameraOnly = (With<MainCamera>, Without<MirrorCamera>);

/// Parameters for [`update_mirror_cameras`].
#[derive(SystemParam)]
pub struct MirrorUpdateParams<'w, 's> {
    pub quality: Res<'w, LightQuality>,
    pub frame: ResMut<'w, MirrorFrame>,
    pub stats: ResMut<'w, MirrorStats>,
    pub settings: Res<'w, MirrorSettings>,
    pub virtual_mirrors: ResMut<'w, VirtualMirrors>,
    pub images: ResMut<'w, Assets<Image>>,
    pub vehicles: Query<'w, 's, &'static GlobalTransform, With<PlayerVehicle>>,
    pub main: Query<'w, 's, MainCameraView<'static>, MainCameraOnly>,
    pub cameras: Query<'w, 's, MirrorCameraQueryItem<'static>>,
    pub panes: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            Option<&'static InheritedVisibility>,
        ),
        With<MirrorSurface>,
    >,
    pub materials_of:
        Query<'w, 's, &'static mut MeshMaterial3d<StandardMaterial>, With<MirrorSurface>>,
}

/// Selects, poses and switches the mirror cameras.
pub fn update_mirror_cameras(mut params: MirrorUpdateParams, mut commands: Commands) {
    params.frame.0 = params.frame.0.wrapping_add(1);
    let live = cleanup_dead_cameras(
        &mut params.cameras,
        &params.panes,
        &params.vehicles,
        &params.quality,
        &params.virtual_mirrors,
        &mut commands,
        &mut params.stats,
    );

    let Some((view, main_projection)) = params.main.iter().next() else {
        for e in &live {
            if let Ok((_, _, _, mut cam, _)) = params.cameras.get_mut(*e) {
                if cam.is_active {
                    cam.is_active = false;
                }
            }
        }
        return;
    };
    let viewer = view.translation();

    let poses = compute_group_poses(&live, &params.cameras, &params.panes, viewer);
    let distances: Vec<f32> = poses.iter().map(|p| p.distance).collect();
    let count = params.quality.mirror_count;
    let in_view = compute_in_view(&poses, count, view, main_projection);

    let streaming = update_virtual_slots(
        &poses,
        &params.cameras,
        &params.vehicles,
        &mut params.virtual_mirrors,
    );

    let last: Vec<u64> = poses
        .iter()
        .map(|p| params.cameras.get(p.camera).map_or(0, |c| c.1.last_render))
        .collect();
    let pickable: Vec<bool> = poses
        .iter()
        .zip(&in_view)
        .map(|(p, v)| *v && !streaming.contains(&p.camera))
        .collect();
    let picked = pick_renders(
        &last,
        &distances,
        &pickable,
        renders_in_frame(params.frame.0, count, params.quality.mirror_every),
    );
    params.stats.selected = poses
        .iter()
        .zip(&in_view)
        .filter(|(p, v)| **v || streaming.contains(&p.camera))
        .count();

    apply_camera_renders_and_poses(&mut params, &poses, &picked, &streaming, count, viewer);
}

type MainCameraLookQuery<'w, 's> = Query<
    'w,
    's,
    (
        Has<Hdr>,
        Option<&'static DistanceFog>,
        Option<&'static Tonemapping>,
    ),
    (
        With<MainCamera>,
        Without<MirrorCamera>,
        Without<AmbientCamera>,
    ),
>;

type MirrorCameraLookItem<'a> = (
    Entity,
    Has<Hdr>,
    Option<&'a DistanceFog>,
    Option<&'a Tonemapping>,
    Option<&'a MirrorCamera>,
);

type MirrorCameraLookFilter = Or<(With<MirrorCamera>, With<AmbientCamera>)>;

/// Copies `Hdr` and `DistanceFog` from the main camera to the mirror cameras (the mirror picture
/// is scene-linear: no tonemapper, no colour grading; the main camera applies its own once). A
/// mirror that streams into a HUD panel takes the main camera's tone mapping too.
pub fn sync_mirror_camera_look(
    main: MainCameraLookQuery,
    mirrors: Query<MirrorCameraLookItem, MirrorCameraLookFilter>,
    mut commands: Commands,
) {
    let Some((hdr, fog, tonemapping)) = main.iter().next() else {
        return;
    };
    for (entity, has_hdr, has_fog, has_tonemapping, mirror) in &mirrors {
        let mut e = commands.entity(entity);
        let want = if mirror.is_some_and(|m| m.enlarged) {
            tonemapping.copied()
        } else {
            Some(Tonemapping::None)
        };
        if let Some(t) = want {
            if has_tonemapping != Some(&t) {
                e.insert(t);
            }
        }
        if hdr && !has_hdr {
            e.insert(Hdr);
        } else if !hdr && has_hdr {
            e.remove::<Hdr>();
        }
        match (fog, has_fog) {
            (Some(f), None) => {
                e.insert(f.clone());
            }
            (Some(f), Some(cur)) if format!("{f:?}") != format!("{cur:?}") => {
                e.insert(f.clone());
            }
            (None, Some(_)) => {
                e.remove::<DistanceFog>();
            }
            _ => {}
        }
    }
}
