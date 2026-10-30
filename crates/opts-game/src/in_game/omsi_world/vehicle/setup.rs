//! Setup helpers of the player vehicle: reading its files, texture definitions, root components,
//! camera and display name.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

use super::{commands_mass, first_ai_list_vehicle, CameraSetup, InGameEntity, WorldLoadJob};

/// Reads the vehicle's `.bus` and its model config: the file the session names (or the first
/// vehicle of the map's AI lists) and the model it points to.
pub(super) fn read_vehicle_files(
    job: &WorldLoadJob,
) -> Result<(PathBuf, BusManifest, PathBuf, ModelConfig), String> {
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
    Ok((bus_path, manifest, cfg_path, model))
}

/// Makes the model's text and script texture definitions the ones the mesh loader sees, with a
/// fresh set of canvases.
pub(super) fn set_texture_defs(job: &mut WorldLoadJob, model: &ModelConfig) {
    job.vehicle_text_defs = model.text_textures.clone();
    job.vehicle_script_defs = model.script_textures.clone();
    let canvases: Vec<opts_core::formats::script_texture::CpuScriptTexture> = model
        .script_textures
        .iter()
        .map(|st| opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height))
        .collect();
    job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
}

/// The simulation and the markers of the vehicle's root: the controller at the spawn point, the
/// session and viewpoint keys, the player marker, the body box and, for rail, the rail player.
pub(super) fn insert_vehicle_components(
    commands: &mut Commands,
    root: Entity,
    (manifest, bus_path): (&BusManifest, &Path),
    (session_id, activate): (crate::session_vehicles::VehicleId, bool),
    (spawn_world, heading_deg): (Vec3, f32),
    formation_free: bool,
) {
    let mut controller = VehicleController::from_manifest(manifest);
    if super::super::is_rail(manifest) {
        // No pedals on rails: drive, the lever at rest (the start-up frees the brake).
        controller
            .inputs_mut()
            .set_gear(opts_sim::GearPosition::Drive);
    }
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
    if let Some(key) = crate::viewpoints::key_of_bus_file(bus_path) {
        commands
            .entity(root)
            .insert(crate::viewpoints::ViewpointKey(key));
    }
    if let Some(model) = crate::viewpoints::model_of_bus_file(bus_path) {
        commands
            .entity(root)
            .insert(crate::viewpoints::ViewpointModel(model));
    }
    if activate {
        commands.entity(root).insert(PlayerVehicle);
    }
    if let Some((size, centre)) = manifest.bounding_box {
        commands.entity(root).insert(super::super::PlayerBox(
            super::super::BodyBox::from_bounding_box(size, centre),
        ));
    }
    if super::super::is_rail(manifest) {
        // Rail-bound: held on the track, speed from the rail dynamics (see `rail.rs`); the
        // coupled units are placed on the track too, not by trailer kinematics.
        let mass = commands_mass(manifest);
        let mut player =
            super::super::RailPlayer::new(mass, manifest.bogie_half_span.unwrap_or(0.0));
        if !formation_free {
            player = player.with_snap(super::super::rail::FORMATION_SNAP_M);
        }
        commands
            .entity(root)
            .insert((player, opts_bevy::RailControls::default()));
    }
}

/// The chase and cockpit camera of the vehicle, from its bounding box; recorded in the job.
pub(super) fn set_camera(
    job: &mut WorldLoadJob,
    manifest: &BusManifest,
    (min, max): (Vec3, Vec3),
    (bevy_pos, rotation): (Vec3, Quat),
    cockpit_offset: Vec3,
) -> CameraSetup {
    tracing::debug!(min = ?min, max = ?max, "vehicle bounding box (Bevy space)");
    let length = (max.z - min.z).abs().max(1.0);
    let height = (max.y - min.y).abs().max(1.0);
    let chase_distance = length * 1.3;
    let chase_height = height * 1.6;
    job.camera_std_index = manifest.std_camera_index.unwrap_or(0);
    let camera = CameraSetup {
        start: Transform::from_translation(
            bevy_pos + rotation * Vec3::new(0.0, chase_height, chase_distance),
        )
        .looking_at(bevy_pos + Vec3::Y * (height * 0.5), Vec3::Y),
        chase_distance,
        chase_height,
        cockpit_offset,
    };
    job.camera = Some(camera);
    camera
}

/// The name the vehicle is known by: manufacturer and model, else the file name.
pub(super) fn vehicle_display_name(manifest: &BusManifest, bus_path: &Path) -> String {
    manifest
        .friendly_name
        .as_ref()
        .map(|f| format!("{} {}", f.manufacturer, f.model))
        .unwrap_or_else(|| {
            bus_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
}
