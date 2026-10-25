//! Passengers and the driver figure.

use super::*;

/// Loads the skinned meshes of a character: the first (highest-detail) LOD of its model manifest.
///
/// Fails with "no human meshes could be loaded" while scrambled meshes are still being decoded in
/// the background; callers may retry later.
pub(in super::super::super) fn load_human_parts(
    job: &mut WorldLoadJob,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    profile: &opts_core::formats::humans::HumanProfile,
    hum_dir: &Path,
) -> Result<Vec<opts_bevy::ModelPart>, String> {
    // Model manifest and skinned meshes (highest-detail LOD).
    let cfg_path = job
        .install
        .find_in(hum_dir, &profile.model)
        .ok_or_else(|| format!("human model config not found: {}", profile.model))?;
    let cfg = opts_core::formats::vehicles::model::ModelConfig::parse_bytes(
        &std::fs::read(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?,
    )
    .map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let model_dir = cfg_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let tex_dirs = vec![
        model_dir.clone(),
        model_dir.join("texture"),
        hum_dir.join("texture"),
        model_dir
            .parent()
            .map(|p| p.join("Texture"))
            .unwrap_or_default(),
    ];
    let lod = cfg.lods.first().ok_or("human model has no meshes")?;
    let mut parts = Vec::new();
    for mesh_def in &lod.meshes {
        let Some(o3d_path) = job.install.find_in(&model_dir, &mesh_def.path) else {
            tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, "human mesh not found");
            continue;
        };
        let overrides: Vec<TextureOverride> = mesh_def
            .materials
            .iter()
            .map(|m| TextureOverride {
                slot: m.slot_index as usize,
                texture: m.texture.clone(),
                alpha: m.alpha_mode,
                alpha_variable: m.alpha_scale_var.clone(),
                trans_map: m.trans_map.clone().filter(|t| !t.trim().is_empty()),
                image: None,
                light_map: None,
                free_texture: None,
                night_map: None,
            })
            .collect();
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        super::super::trace::mesh_used(&o3d_path);
        parts.extend(loader.o3d_skinned_parts(&o3d_path, &tex_dirs, &overrides));
    }
    if parts.is_empty() {
        return Err("no human meshes could be loaded".into());
    }
    Ok(parts)
}

/// Loads the player's character and seats it at the vehicle's `[drivpos]`.
///
/// The skinned mesh, joint pivots and head anchor come from the `.hum`, its model manifest and
/// the `.o3d` bone weights; the seat contact point comes from the vehicle's passenger cabin.
#[allow(clippy::too_many_arguments)]
pub(in super::super::super) fn load_driver_human(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    vehicle_root: Entity,
    bus_dir: &Path,
    cabin_rel: Option<&str>,
    eye_target: Vec3,
) -> Result<String, String> {
    use opts_bevy::{hip_offset, spawn_human, HumanPose};
    use opts_core::formats::humans::HumanProfile;
    use opts_core::formats::text::{decode_bytes, scan_blocks};

    let hum_path = job
        .config
        .human_path
        .clone()
        .ok_or("no .hum profile in the installation")?;
    let profile = HumanProfile::parse(
        &std::fs::read(&hum_path).map_err(|e| format!("{}: {e}", hum_path.display()))?,
    )
    .map_err(|e| format!("{}: {e}", hum_path.display()))?;
    let hum_dir = hum_path.parent().map(Path::to_path_buf).unwrap_or_default();

    // Driver seat: `[drivpos]` x, y, z, h, rot in the vehicle's passenger cabin.
    let cabin = cabin_rel
        .and_then(|c| job.install.find_in(bus_dir, c))
        .ok_or("vehicle has no passenger cabin file")?;
    let text =
        decode_bytes(&std::fs::read(&cabin).map_err(|e| format!("{}: {e}", cabin.display()))?);
    let blocks = scan_blocks(&text);
    let seat: Vec<f32> = blocks
        .iter()
        .find(|b| b.keyword == "drivpos")
        .ok_or("passenger cabin has no [drivpos]")?
        .parameters
        .iter()
        .take(5)
        .map(|p| {
            p.trim()
                .replace(',', ".")
                .parse::<f32>()
                .map_err(|_| format!("bad [drivpos] value '{p}'"))
        })
        .collect::<Result<_, _>>()?;
    if seat.len() < 5 {
        return Err("[drivpos] needs 5 values".into());
    }

    let parts = load_human_parts(job, meshes, materials, images, &profile, &hum_dir)?;

    tracing::info!(parts = parts.len(), vertices = parts.iter().map(|p| p.vertices).sum::<usize>(), seat = ?seat, "human meshes loaded");
    let name = hum_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let human = spawn_human(
        commands,
        bindposes,
        &profile,
        &parts,
        &name,
        HumanPose::Seated,
    );
    // Hips on the seat contact point, facing along `rot` (clockwise like headings).
    let contact = omsi_to_bevy_pos(Vec3::new(seat[0], seat[1], seat[2]));
    commands.entity(human).insert(Transform {
        translation: contact - hip_offset(&profile),
        rotation: omsi_euler_to_bevy_quat(-seat[4], 0.0, 0.0),
        scale: Vec3::ONE,
    });
    commands.entity(vehicle_root).add_child(human);
    // The eyes go where the vehicle's driver camera is (set once the camera is known).
    commands.entity(human).insert(opts_bevy::AlignEyes {
        target: eye_target,
        wait: 3,
    });
    Ok(name)
}
