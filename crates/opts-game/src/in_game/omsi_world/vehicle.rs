//! Player vehicle, trailers and driver loading.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::coords::{
    camera_preset_to_bevy_transform, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
};
use opts_bevy::{
    attach_model_parts, spawn_vehicle_model, ContentLoader, PlayerVehicle, TextureOverride,
    VehicleController,
};
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

use crate::in_game::types::InGameEntity;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{CameraSetup, WorldLoadJob, WorldStats};

mod doors;
mod segments;
use segments::{ensure_repaints, repaint_map};
pub(super) use segments::{first_ai_list_vehicle, load_segment_meshes, load_segment_meshes_for};

mod skins;

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
#[allow(clippy::too_many_arguments)]
pub fn load_player_vehicle(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    session_id: crate::session_vehicles::VehicleId,
    activate: bool,
) -> Result<SpawnedVehicle, String> {
    let bus_path = match job.config.vehicle_path.clone() {
        Some(p) => p,
        None => first_ai_list_vehicle(job)
            .ok_or("no --vehicle-path given and the map's ailists.cfg lists no vehicle")?,
    };
    let bus_bytes = std::fs::read(&bus_path).map_err(|e| format!("{}: {e}", bus_path.display()))?;
    let manifest =
        BusManifest::parse_bytes(&bus_bytes).map_err(|e| format!("{}: {e}", bus_path.display()))?;
    tracing::info!(path = %bus_path.display(), mass_t = ?manifest.mass_tonnes, axles = manifest.axles.len(), "vehicle .bus parsed");

    let bus_dir = bus_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let cfg_rel = manifest
        .model_cfg
        .clone()
        .ok_or("the .bus file has no [model] entry")?;
    let cfg_path = job
        .install
        .find_in(&bus_dir, &cfg_rel)
        .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
    let cfg_bytes = std::fs::read(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let model =
        ModelConfig::parse_bytes(&cfg_bytes).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let detail = model.detail_lod().ok_or("model config has no meshes")?;
    let lod0 = &detail;
    job.vehicle_text_defs = model.text_textures.clone();
    job.vehicle_script_defs = model.script_textures.clone();
    let canvases: Vec<opts_core::formats::script_texture::CpuScriptTexture> = model
        .script_textures
        .iter()
        .map(|st| opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height))
        .collect();
    job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
    tracing::info!(path = %cfg_path.display(), lods = model.lods.len(), meshes = lod0.meshes.len(), "vehicle model config parsed");

    let model_dir = cfg_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let tex_dirs = vec![
        job.install
            .find_in(&bus_dir, "texture")
            .unwrap_or_else(|| bus_dir.join("texture")),
        model_dir.join("texture"),
    ];

    // Spawn point: the start road spline's lane position, converted to Bevy space.
    let (spawn_world, heading_deg) = job.spawn.ok_or("no spawn point")?;
    let bevy_pos = omsi_to_bevy_pos(spawn_world);
    let root_tf = Transform {
        translation: bevy_pos,
        rotation: omsi_euler_to_bevy_quat(-heading_deg, 0.0, 0.0),
        scale: Vec3::ONE,
    };

    job.vehicle_dirs = std::iter::once(bus_dir.clone())
        .chain(std::iter::once(model_dir.clone()))
        .chain(tex_dirs.iter().cloned())
        .map(|d| opts_bevy::hot_reload::cache::normalize(&d))
        .collect();
    let root = spawn_vehicle_model(commands, &manifest, &model, root_tf, &[], &[]);

    ensure_repaints(job, &bus_path, &manifest);
    job.cache.texture_replacements = repaint_map(job, &model);
    super::trace::begin_frame();
    let (mesh_count, vertex_count, min, max) = load_segment_meshes(
        job, commands, meshes, materials, images, root, lod0, &model_dir, &tex_dirs,
    );
    let trace_frame = super::trace::end_frame();
    job.cache.texture_replacements.clear();
    if mesh_count == 0 {
        return Err("no vehicle meshes could be loaded".into());
    }

    let mut controller = VehicleController::from_manifest(&manifest);
    controller.sim.chassis.position = Vec2::new(spawn_world.x, spawn_world.y);
    controller.sim.chassis.elevation_z = spawn_world.z;
    // The simulation uses compass-style heading (clockwise from +Y), like OMSI map headings.
    controller.sim.chassis.heading_rad = heading_deg.to_radians();
    commands.entity(root).insert((
        controller,
        crate::session_vehicles::SessionVehicle(session_id),
        InGameEntity,
    ));
    // The catalogue identity of the vehicle keys its stored viewpoints.
    if let Some(key) = crate::viewpoints::key_of_bus_file(&bus_path) {
        commands
            .entity(root)
            .insert(crate::viewpoints::ViewpointKey(key));
    }
    if let Some(model) = crate::viewpoints::model_of_bus_file(&bus_path) {
        commands
            .entity(root)
            .insert(crate::viewpoints::ViewpointModel(model));
    }
    if activate {
        commands.entity(root).insert(PlayerVehicle);
    }
    if let Some((size, centre)) = manifest.bounding_box {
        commands
            .entity(root)
            .insert(super::PlayerBox(super::BodyBox::from_bounding_box(
                size, centre,
            )));
    }
    if super::is_rail(&manifest) {
        // Rail-bound: held on the track, speed from the rail dynamics (see `rail.rs`); the
        // coupled units are placed on the track too, not by trailer kinematics.
        let mass = commands_mass(&manifest);
        let mut player = super::RailPlayer::new(mass, manifest.bogie_half_span.unwrap_or(0.0));
        if !job.formation_cars.is_empty() {
            player = player.with_snap(super::rail::FORMATION_SNAP_M);
        }
        commands.entity(root).insert(player);
    }
    let script_seed = script_seed(job.install.root(), &bus_path, session_id);
    attach_scripts(job, commands, root, &manifest, &bus_dir, script_seed);
    crate::player_sound::attach(job, commands, root, &manifest, &bus_dir);
    doors::attach_door_points(job, commands, root, &manifest, &bus_dir);
    load_key_bindings(job, commands);
    let trailer_cams = spawn_trailer_chain(
        job, commands, meshes, materials, images, root, &bus_dir, &manifest,
    );
    if !trailer_cams.is_empty() {
        commands.queue(move |world: &mut World| {
            if let Some(mut bus) = world.get_mut::<opts_bevy::BusVehicle>(root) {
                bus.trailer_pax_cameras = trailer_cams;
            }
        });
    }
    let driver_cam = manifest
        .driver_cameras
        .get(manifest.std_camera_index.unwrap_or(0))
        .or_else(|| manifest.driver_cameras.first());
    // Driver camera preset converted by the shared helper (vehicle space → Bevy).
    let cockpit_offset = driver_cam
        .map(|c| {
            camera_preset_to_bevy_transform(
                Vec3::new(c.x, c.y, c.z),
                c.dist,
                c.yaw_deg,
                c.pitch_deg,
            )
            .translation
        })
        .unwrap_or(Vec3::new(0.0, 1.2, 0.0));
    match load_driver_human(
        job,
        commands,
        meshes,
        materials,
        images,
        bindposes,
        root,
        &bus_dir,
        manifest.passengercabin_cfg.as_deref(),
        cockpit_offset,
    ) {
        Ok(name) => tracing::info!(human = %name, "player character seated at the driver position"),
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), "cannot load the player character: {e}")
        }
    }

    tracing::debug!(min = ?min, max = ?max, "vehicle bounding box (Bevy space)");
    let length = (max.z - min.z).abs().max(1.0);
    let height = (max.y - min.y).abs().max(1.0);
    let chase_distance = length * 1.3;
    let chase_height = height * 1.6;
    job.camera_std_index = manifest.std_camera_index.unwrap_or(0);
    let camera = CameraSetup {
        start: Transform::from_translation(
            bevy_pos + root_tf.rotation * Vec3::new(0.0, chase_height, chase_distance),
        )
        .looking_at(bevy_pos + Vec3::Y * (height * 0.5), Vec3::Y),
        chase_distance,
        chase_height,
        cockpit_offset,
    };
    job.camera = Some(camera);

    let vehicle_name = manifest
        .friendly_name
        .as_ref()
        .map(|f| format!("{} {}", f.manufacturer, f.model))
        .unwrap_or_else(|| {
            bus_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
    if activate {
        stats.vehicle_name.clone_from(&vehicle_name);
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
    if opts_bevy::scene_trace::enabled() {
        super::trace::spawned(
            opts_bevy::scene_trace::kind::VEHICLE,
            &vehicle_name,
            &opts_bevy::scene_trace::relative(&bus_path),
            root,
            &root_tf,
            &trace_frame,
            "ok",
            serde_json::json!({
                "session_id": session_id, "active": activate,
                "model_cfg": opts_bevy::scene_trace::relative(&cfg_path),
                "meshes": mesh_count, "vertices": vertex_count,
            }),
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
