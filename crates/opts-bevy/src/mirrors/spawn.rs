//! Creating the mirror cameras and their surfaces for a vehicle.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::glass::mirror_index;
use crate::mirrors::*;

mod group;
mod material;
mod scan;

use group::{create_mirror_camera, join_existing_group};
use scan::{scan_vehicle_panes, CandidateMeshQuery};

pub(super) use material::mirror_material;

/// Parameters for [`spawn_mirror_cameras`].
#[derive(SystemParam)]
pub struct MirrorSpawnParams<'w, 's> {
    pub time: Res<'w, Time<Real>>,
    pub quality: Res<'w, LightQuality>,
    pub settings: Res<'w, MirrorSettings>,
    pub images: ResMut<'w, Assets<Image>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub virtual_mirrors: Res<'w, VirtualMirrors>,
    pub meshes: Res<'w, Assets<Mesh>>,
    pub vehicles: Query<'w, 's, Entity, With<PlayerVehicle>>,
    pub buses: Query<'w, 's, &'static BusVehicle>,
    pub children: Query<'w, 's, &'static Children>,
    pub cameras: Query<'w, 's, (Entity, &'static mut MirrorCamera)>,
    pub candidates: CandidateMeshQuery<'w, 's>,
}

/// Finds the mirror panes of the player vehicle and gives each group of panes a camera.
pub fn spawn_mirror_cameras(
    mut params: MirrorSpawnParams,
    mut last_scan: Local<Option<f64>>,
    mut commands: Commands,
) {
    if params.quality.mirror_count == 0 && !params.virtual_mirrors.needs_cameras() {
        return;
    }
    let now = params.time.elapsed_secs_f64();
    if last_scan.is_some_and(|t| now - t < params.settings.scan_interval_s) {
        return;
    }
    *last_scan = Some(now);

    let vehicles: Vec<Entity> = params.vehicles.iter().collect();
    for vehicle in vehicles {
        let groups = scan_vehicle_panes(
            vehicle,
            &params.children,
            &params.candidates,
            &params.meshes,
        );
        let mut existing: HashMap<GroupKey, Entity> = params
            .cameras
            .iter()
            .filter(|(_, c)| c.vehicle == vehicle)
            .map(|(e, c)| (c.key, e))
            .collect();
        for (key, (original, found)) in groups {
            if let Some(camera) = existing.get(&key).copied() {
                join_existing_group(
                    camera,
                    found,
                    &mut params.cameras,
                    &mut params.materials,
                    &params.settings,
                    &mut commands,
                );
                continue;
            }
            if existing.len() >= MAX_CAMERAS {
                continue;
            }
            let tag = if params.settings.use_tags {
                match key {
                    GroupKey::Index(n) => Some(n),
                    GroupKey::Material(id) => mirror_index(id),
                }
                .and_then(|n| {
                    params
                        .buses
                        .get(vehicle)
                        .ok()
                        .and_then(|b| b.mirror_cameras.get(n as usize).cloned())
                })
            } else {
                None
            };
            if let Some(camera) = create_mirror_camera(
                vehicle,
                key,
                original,
                found,
                tag,
                &mut params,
                &mut commands,
            ) {
                existing.insert(key, camera);
            }
        }
    }
}
