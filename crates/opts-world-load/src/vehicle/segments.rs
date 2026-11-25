//! Loading the segment meshes of a vehicle and its repaints.

use super::*;

mod joints;
mod mesh;
mod overrides;
mod parts;
mod state;
mod trailer;
use super::skins::TrailerLink;
use state::{SegmentAssets, SegmentState};

/// Reads the first vehicle listed in the map's `ailists.cfg` (fallback when no `--vehicle-path`).
pub(in super::super::super) fn first_ai_list_vehicle(job: &WorldLoadJob) -> Option<PathBuf> {
    let cfg = job.install.find_in(&job.config.map_dir, "ailists.cfg")?;
    let text = opts_core::formats::text::decode_bytes(&std::fs::read(cfg).ok()?);
    text.lines()
        .map(str::trim)
        .filter(|l| l.to_lowercase().contains(".bus"))
        .find_map(|l| {
            let rel = l.split('\t').next()?.trim();
            job.install.find_file(rel)
        })
}

/// Reads the repaints of the player's vehicle once per session (they come from the `.cti` files of
/// its `Colorscheme` folders, `docs/src/formats/text/repaints.md`).
pub(super) fn ensure_repaints(job: &mut WorldLoadJob, bus_path: &Path, manifest: &BusManifest) {
    if job.repaint.is_none() {
        let default = manifest
            .friendly_name
            .as_ref()
            .map(|f| f.livery.clone())
            .unwrap_or_default();
        job.repaint = Some(opts_repaints::discover_at(
            &job.config.root,
            bus_path,
            &default,
        ));
    }
}

/// Texture replacements of the chosen repaint for the model of one segment: each `[CTCTexture]`
/// slot's default file is swapped for the repaint's file of that slot.
pub(super) fn repaint_map(
    job: &WorldLoadJob,
    model: &ModelConfig,
) -> std::collections::HashMap<String, PathBuf> {
    let Some(r) = &job.repaint else {
        return Default::default();
    };
    let name = if job.config.livery.is_empty() {
        r.default_name.as_str()
    } else {
        job.config.livery.as_str()
    };
    r.replacements(name, &model.ctc_textures)
        .into_iter()
        .collect()
}

/// Loads the visible lod0 meshes of one vehicle segment as children of `root`.
/// Returns `(mesh_count, vertex_count, bounds_min, bounds_max)` (Bevy space, body meshes only).
pub(in super::super::super) fn load_segment_meshes(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    stores: (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    root: Entity,
    lod0: &opts_core::formats::vehicles::model::ModelLod,
    model_dir: &Path,
    tex_dirs: &[PathBuf],
) -> (usize, usize, Vec3, Vec3) {
    load_segment_meshes_for(
        job,
        commands,
        stores,
        (root, root),
        lod0,
        model_dir,
        tex_dirs,
    )
}

/// [`load_segment_meshes`] for a segment whose animations, light maps and hot spots read the
/// variables of `owner` (the leading unit: trailers share its scripts, `[scriptshare]`).
pub(in super::super::super) fn load_segment_meshes_for(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (root, owner): (Entity, Entity),
    lod0: &opts_core::formats::vehicles::model::ModelLod,
    model_dir: &Path,
    tex_dirs: &[PathBuf],
) -> (usize, usize, Vec3, Vec3) {
    load_segment_linked(
        job,
        commands,
        (meshes, materials, images),
        (root, owner),
        lod0,
        (model_dir, tex_dirs),
        None,
    )
}

/// [`load_segment_meshes`] for the head of an articulated vehicle: the bellow's trailer end
/// follows the coupled `trailer` (see [`TrailerLink`]), when there is one.
pub(in super::super::super) fn load_head_segment_meshes(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    stores: (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    root: Entity,
    lod0: &opts_core::formats::vehicles::model::ModelLod,
    (model_dir, tex_dirs): (&Path, &[PathBuf]),
    trailer: Option<TrailerLink>,
) -> (usize, usize, Vec3, Vec3) {
    load_segment_linked(
        job,
        commands,
        stores,
        (root, root),
        lod0,
        (model_dir, tex_dirs),
        trailer,
    )
}

fn load_segment_linked(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (root, owner): (Entity, Entity),
    lod0: &opts_core::formats::vehicles::model::ModelLod,
    (model_dir, tex_dirs): (&Path, &[PathBuf]),
    trailer: Option<TrailerLink>,
) -> (usize, usize, Vec3, Vec3) {
    let mut assets = SegmentAssets {
        meshes,
        materials,
        images,
    };
    let vehicle_part = std::mem::replace(&mut job.cache.vehicle_part, true);
    let mut state = SegmentState::new(commands, (root, owner), lod0, (model_dir, tex_dirs));
    state.trailer = trailer;
    let counts = state.load_all(job, commands, &mut assets);
    job.cache.vehicle_part = vehicle_part;
    counts
}

#[cfg(test)]
mod tests;
