//! Step handlers for a [`TileBuild`](super::TileBuild) in progress: the phases are split by
//! concern into [`ground`], [`surfaces`] and [`objects`]; the last one, `finish`, lives here.

mod ground;
mod objects;
mod surfaces;

pub(super) use ground::{splines_step, terrain_step};
pub(super) use objects::objects_step;
pub(super) use surfaces::{attachments_step, registries_step, surfaces_step};

use super::super::tile_ground::prune_tile_surfaces;
use super::*;

pub(super) fn finish(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
) {
    if opts_bevy::instancing::merge() {
        let merged = job.cache.merger.flush(commands, meshes);
        b.children.extend(merged);
        let st = job.cache.merger.stats;
        tracing::info!(
            tile = ?b.key,
            parts_merged = st.parts_merged,
            entities = st.entities,
            refused = st.refused,
            "static scenery merged (running totals)"
        );
    }
    let _t = opts_logging::stream_trace::span("tile.root_adopt");
    let deps = std::mem::take(&mut job.loading_deps);
    job.tile_deps.insert(b.key, deps);
    prune_tile_surfaces(job);
    stats.tiles_loaded = job.resident.len();
    stats.meshes_exact = job.cache.stats.exact_meshes;
    stats.asset_failures = job.cache.stats.failures.len();
}
