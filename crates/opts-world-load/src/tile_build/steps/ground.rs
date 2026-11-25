//! Ground steps of a [`TileBuild`](super::super::TileBuild): the terrain and the water, then the
//! splines on it.

use super::super::super::tile_ground::{spawn_splines, spawn_terrain, spawn_water};
use super::super::*;

pub(in super::super) fn terrain_step(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let _t = opts_logging::stream_trace::span("tile.terrain");
    b.terrain = spawn_terrain(
        job,
        stats,
        commands,
        meshes,
        b.terrain_input.take(),
        (b.key, b.origin),
        &mut b.children,
    );
    spawn_water(
        (job, stats, commands),
        (meshes, materials),
        (&b.tile, b.terrain.as_ref(), b.key, b.origin),
        &mut b.children,
    );
    b.splines_failed_before = stats.splines_failed;
    b.phase = Phase::Splines;
}

pub(in super::super) fn splines_step(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let _t = opts_logging::stream_trace::span("tile.splines");
    spawn_splines(
        (job, stats, commands),
        (meshes, materials, images),
        (&b.tile, b.origin),
        &mut b.children,
    );
    if opts_bevy::scene_trace::enabled() {
        let failed = stats.splines_failed.saturating_sub(b.splines_failed_before);
        super::super::super::trace::tile_content(
            job,
            &b.tile,
            b.key,
            b.origin,
            b.terrain.is_some(),
            failed,
        );
    }
    b.phase = Phase::Surfaces;
}
