//! Loading the characters of the map's `humans.txt`, one slot per distinct character.

use super::*;

/// Reads the map's `humans.txt` and makes a slot for each distinct character.
pub(super) fn start_pool(job: &WorldLoadJob, pool: &mut PedestrianPool) {
    pool.started = true;
    pool.rng = 0x9E37_79B9;
    let text = job
        .install
        .find_in(&job.config.map_dir, "humans.txt")
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| opts_core::formats::text::decode_bytes(&b))
        .unwrap_or_default();
    pool.entries = text
        .lines()
        .map(str::trim)
        .filter(|l| l.to_ascii_lowercase().ends_with(".hum"))
        .map(str::to_string)
        .collect();
    let mut distinct: Vec<String> = Vec::new();
    for e in &pool.entries {
        if !distinct.iter().any(|d| d.eq_ignore_ascii_case(e)) {
            distinct.push(e.clone());
        }
    }
    pool.slots = distinct.into_iter().map(|d| (d, None, 0)).collect();
    if pool.slots.is_empty() {
        tracing::error!(at = %opts_bevy::position_tag(), map = %job.config.map_dir.display(), "no characters in humans.txt; no pedestrians");
    }
}

/// Stores the loaded character in its slot, or gives the slot up when the mesh has no bone
/// weights (a skinned draw call with a non-skinned mesh is a render validation error). A model that
/// is not ready yet is tried again later.
pub(super) fn store_variant(
    slot: &mut (String, Option<Variant>, u32),
    (path, profile): (&std::path::Path, HumanProfile),
    (loaded_parts, trace_frame): (
        Result<Vec<opts_bevy::ModelPart>, String>,
        &super::super::trace::Frame,
    ),
    meshes: &Assets<Mesh>,
) {
    match loaded_parts {
        Ok(parts)
            if parts.iter().any(|p| {
                meshes
                    .get(&p.mesh)
                    .is_some_and(|m| m.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_none())
            }) =>
        {
            // Without bone weights the mesh cannot follow the skeleton (and a skinned draw call
            // with a non-skinned mesh is a render validation error).
            tracing::error!(at = %opts_bevy::position_tag(), hum = %path.display(), "character mesh has no bone weights; not used as pedestrian");
            slot.2 = 40;
        }
        Ok(parts) => {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            tracing::info!(character = %name, parts = parts.len(), "pedestrian model loaded");
            super::super::trace::remember_human(&name, trace_frame);
            slot.1 = Some(Variant {
                name,
                profile,
                parts,
            });
        }
        Err(e) => {
            // Scrambled meshes decode in the background; try again later.
            tracing::debug!(hum = %path.display(), "pedestrian model not ready: {e}");
        }
    }
}

/// Reads `humans.txt` once and loads one missing character model every few frames.
pub fn load_variants(
    job: Option<ResMut<WorldLoadJob>>,
    settings: Res<PedestrianSettings>,
    mut pool: ResMut<PedestrianPool>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut job) = job else { return };
    if settings.baseline == 0 && !pool.wanted {
        return;
    }
    if !pool.started {
        start_pool(&job, &mut pool);
    }
    pool.frames += 1;
    if !pool.frames.is_multiple_of(20) {
        return;
    }
    let Some(i) = pool
        .slots
        .iter()
        .position(|(_, v, tries)| v.is_none() && *tries < 40)
    else {
        return;
    };
    pool.slots[i].2 += 1;
    let rel = pool.slots[i].0.clone();
    let Some(path) = job.install.find_file(&rel) else {
        tracing::error!(at = %opts_bevy::position_tag(), hum = %rel, "pedestrian character not found");
        pool.slots[i].2 = 40;
        return;
    };
    let profile = match std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|b| HumanProfile::parse(&b).map_err(|e| e.to_string()))
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), hum = %path.display(), "pedestrian profile failed: {e}");
            pool.slots[i].2 = 40;
            return;
        }
    };
    let dir = path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    super::super::trace::begin_frame();
    let loaded_parts = load_human_parts(
        &mut job,
        &mut meshes,
        &mut materials,
        &mut images,
        &profile,
        &dir,
    );
    let trace_frame = super::super::trace::end_frame();
    store_variant(
        &mut pool.slots[i],
        (&path, profile),
        (loaded_parts, &trace_frame),
        &meshes,
    );
}
