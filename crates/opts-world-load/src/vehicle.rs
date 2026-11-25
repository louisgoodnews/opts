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
mod report;
use report::{record_vehicle, trace_vehicle};
mod attach;
use attach::{attach_trailers, attach_vehicle_parts};
mod driver;
use driver::{cockpit_offset_of, seat_driver};
mod passengers;
mod segments;
use segments::{ensure_repaints, repaint_map};
pub(super) use segments::{
    first_ai_list_vehicle, load_head_segment_meshes, load_segment_meshes, load_segment_meshes_for,
};

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
    let tex_dirs = opts_repaints::texture_dirs(&bus_dir, &model_dir);

    let (spawn_world, heading_deg, root_tf) = spawn_pose(job)?;
    let bevy_pos = root_tf.translation;
    set_vehicle_dirs(job, (&bus_dir, &model_dir, &tex_dirs));
    let root = spawn_vehicle_model(commands, &manifest, &model, root_tf, &[], &[]);

    ensure_repaints(job, &bus_path, &manifest);
    job.cache.texture_replacements = repaint_map(job, &model);
    super::trace::begin_frame();
    let trailer = skins::trailer_link(job, &bus_dir, &manifest);
    let (mesh_count, vertex_count, min, max) = load_head_segment_meshes(
        job,
        commands,
        (&mut *meshes, &mut *materials, &mut *images),
        root,
        lod0,
        (&model_dir, &tex_dirs),
        trailer,
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
