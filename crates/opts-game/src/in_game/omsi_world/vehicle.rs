//! Player vehicle, trailers and driver loading.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::coords::{
    camera_preset_to_bevy_transform, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
};
use opts_bevy::{attach_model_parts, spawn_vehicle_model, ContentLoader, TextureOverride};
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

use crate::in_game::types::InGameEntity;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{CameraSetup, WorldLoadJob, WorldStats};

pub(super) mod doors;
mod hot_spots;
mod passengers;
mod segments;
use segments::{ensure_repaints, repaint_map};
pub(super) use segments::{first_ai_list_vehicle, load_segment_meshes, load_segment_meshes_for};

mod setup;
use setup::{
    insert_vehicle_components, read_vehicle_files, set_camera, set_texture_defs,
    vehicle_display_name,
};
mod assemble;
mod skins;
use assemble::{assemble_vehicle, set_vehicle_dirs, spawn_pose};

mod trailers;
use trailers::commands_mass;
pub use trailers::FormationCar;
pub(super) use trailers::{coupling_joint, spawn_trailer_chain};

/// A vehicle [`load_player_vehicle`] built.
#[derive(Debug, Clone)]
pub struct SpawnedVehicle {
    /// The vehicle's root entity.
    pub root: Entity,
    /// Display name (manufacturer and model, else the `.bus` file's name).
    pub name: String,
    /// Camera parameters derived from the vehicle.
    pub camera: CameraSetup,
}

/// What the world statistics say about the new vehicle.
fn record_vehicle(
    (job, stats): (&WorldLoadJob, &mut WorldStats),
    (vehicle_name, activate): (&str, bool),
    (session_id, mesh_count, vertex_count): (crate::session_vehicles::VehicleId, usize, usize),
) {
    if activate {
        stats.vehicle_name.clone_from(&vehicle_name.to_owned());
    }
    stats.vehicle_meshes = mesh_count;
    stats.vehicle_vertices = vertex_count;
    stats.scene_vertices += vertex_count;
    stats.asset_failures = job.cache.stats.failures.len();
    tracing::info!(
        vehicle = %vehicle_name,
        session_id,
        meshes = mesh_count,
        vertices = vertex_count,
        "vehicle meshes loaded"
    );
}

/// The scene trace record of the new vehicle.
fn trace_vehicle(
    (vehicle_name, bus_path, cfg_path): (&str, &Path, &Path),
    (root, root_tf, trace_frame): (Entity, &Transform, &super::trace::Frame),
    (session_id, activate, mesh_count, vertex_count): (
        crate::session_vehicles::VehicleId,
        bool,
        usize,
        usize,
    ),
) {
    super::trace::spawned(
        opts_bevy::scene_trace::kind::VEHICLE,
        vehicle_name,
        &opts_bevy::scene_trace::relative(bus_path),
        (root, root_tf),
        trace_frame,
        "ok",
        serde_json::json!({
            "session_id": session_id, "active": activate,
            "model_cfg": opts_bevy::scene_trace::relative(cfg_path),
            "meshes": mesh_count, "vertices": vertex_count,
        }),
    );
}

/// The scripts, sounds, door points, passengers and key bindings of the vehicle.
fn attach_vehicle_parts(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    (manifest, bus_dir): (&BusManifest, &Path),
    (script_seed, activate): (u64, bool),
) {
    attach_scripts(job, commands, root, manifest, bus_dir, script_seed, None);
    crate::player_sound::attach(job, commands, root, manifest, bus_dir);
    doors::attach_door_points(job, commands, root, manifest, bus_dir);
    if activate {
        passengers::attach_passengers(job, commands, root, manifest, bus_dir, script_seed);
    }
    load_key_bindings(job, commands);
}

/// The trailing units of an articulated vehicle and the cameras they bring.
fn attach_trailers(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    root: Entity,
    (bus_dir, manifest): (&Path, &BusManifest),
) {
    let (trailer_cams, trailer_driver_cams) = spawn_trailer_chain(
        job,
        commands,
        meshes,
        materials,
        images,
        root,
        (bus_dir, manifest),
    );
    if !trailer_cams.is_empty() || !trailer_driver_cams.is_empty() {
        commands.queue(move |world: &mut World| {
            if let Some(mut bus) = world.get_mut::<opts_bevy::BusVehicle>(root) {
                bus.trailer_pax_cameras = trailer_cams;
                bus.trailer_driver_cameras = trailer_driver_cams;
            }
        });
    }
}

/// The driver's eye point of the standard camera, in Bevy space (vehicle space → Bevy by the
/// shared helper).
fn cockpit_offset_of(manifest: &BusManifest) -> Vec3 {
    let driver_cam = manifest
        .driver_cameras
        .get(manifest.std_camera_index.unwrap_or(0))
        .or_else(|| manifest.driver_cameras.first());
    driver_cam
        .map(|c| {
            camera_preset_to_bevy_transform(
                Vec3::new(c.x, c.y, c.z),
                c.dist,
                c.yaw_deg,
                c.pitch_deg,
            )
            .translation
        })
        .unwrap_or(Vec3::new(0.0, 1.2, 0.0))
}

/// Seats the player's character at the driver position.
fn seat_driver(
    (job, commands): (&mut WorldLoadJob, &mut Commands),
    (meshes, materials, images, bindposes): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
        &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    ),
    root: Entity,
    (bus_dir, manifest, cockpit_offset): (&Path, &BusManifest, Vec3),
) {
    match load_driver_human(
        (job, commands),
        (meshes, materials, images, bindposes),
        root,
        (
            bus_dir,
            manifest.passengercabin_cfg.as_deref(),
            cockpit_offset,
        ),
    ) {
        Ok(name) => tracing::info!(human = %name, "player character seated at the driver position"),
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), "cannot load the player character: {e}")
        }
    }
}

/// Loads a user vehicle from its `.bus` file and spawns it at the session's spawn point
/// ([`WorldLoadJob::spawn_point`]), as session vehicle `session_id`. With `activate` it also becomes the
/// player's vehicle (`PlayerVehicle`, the HUD and `WorldStats` follow it); without, it is built
/// as a parked session vehicle and the caller switches to it when it wants to.
pub fn load_player_vehicle(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials, images, bindposes): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
        &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    ),
    (session_id, activate): (crate::session_vehicles::VehicleId, bool),
) -> Result<SpawnedVehicle, String> {
    let (bus_path, manifest, cfg_path, model) = read_vehicle_files(job)?;
    let bus_dir = bus_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let detail = model.detail_lod().ok_or("model config has no meshes")?;
    let lod0 = &detail;
    set_texture_defs(job, &model);
    tracing::info!(path = %cfg_path.display(), lods = model.lods.len(), meshes = lod0.meshes.len(), "vehicle model config parsed");

    let model_dir = super::mesh_dir::model_mesh_dir(&job.install, &bus_dir, &cfg_path, &model);
    let tex_dirs = crate::new_game_screen::repaints::texture_dirs(&bus_dir, &model_dir);

    let (spawn_world, heading_deg, root_tf) = spawn_pose(job)?;
    let bevy_pos = root_tf.translation;
    set_vehicle_dirs(job, (&bus_dir, &model_dir, &tex_dirs));
    let root = spawn_vehicle_model(commands, &manifest, &model, root_tf, &[], &[]);

    ensure_repaints(job, &bus_path, &manifest);
    job.cache.texture_replacements = repaint_map(job, &model);
    super::trace::begin_frame();
    let (mesh_count, vertex_count, min, max) = load_segment_meshes(
        job,
        commands,
        (&mut *meshes, &mut *materials, &mut *images),
        root,
        lod0,
        &model_dir,
        &tex_dirs,
    );
    let trace_frame = super::trace::end_frame();
    job.cache.texture_replacements.clear();
    if mesh_count == 0 {
        return Err("no vehicle meshes could be loaded".into());
    }

    let cockpit_offset = assemble_vehicle(
        (job, commands),
        (meshes, materials, images, bindposes),
        root,
        (&manifest, &bus_path, &bus_dir),
        (session_id, activate, (spawn_world, heading_deg)),
    );

    let camera = set_camera(
        job,
        &manifest,
        (min, max),
        (bevy_pos, root_tf.rotation),
        cockpit_offset,
    );
    let vehicle_name = vehicle_display_name(&manifest, &bus_path);
    record_vehicle(
        (job, stats),
        (&vehicle_name, activate),
        (session_id, mesh_count, vertex_count),
    );
    if opts_bevy::scene_trace::enabled() {
        trace_vehicle(
            (&vehicle_name, &bus_path, &cfg_path),
            (root, &root_tf, &trace_frame),
            (session_id, activate, mesh_count, vertex_count),
        );
    }
    Ok(SpawnedVehicle {
        root,
        name: vehicle_name,
        camera,
    })
}

mod humans;
pub(super) use humans::{load_driver_human, load_human_parts};

mod scripts;
use scripts::{attach_scripts, load_key_bindings, script_seed};

#[cfg(test)]
mod bone_tests;

#[cfg(test)]
mod consist_mass_tests;

#[cfg(test)]
mod formation_tests;

#[cfg(test)]
mod anim_parent_tests;

#[cfg(test)]
mod seed_tests;
