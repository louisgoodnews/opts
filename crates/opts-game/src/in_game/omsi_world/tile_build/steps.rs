//! Step handlers for a [`TileBuild`](super::TileBuild) in progress.

use crate::in_game::attachments::expand_attachments;

use super::super::tile::{spawn_object, Placement, SURFACE_MIN_WIDTH_M};
use super::super::tile_ground::{prune_tile_surfaces, spawn_splines, spawn_terrain, spawn_water};
use super::*;

pub(super) fn terrain_step(
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

pub(super) fn splines_step(
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
        super::super::trace::tile_content(
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

pub(super) fn surfaces_step(b: &mut TileBuild, job: &mut WorldLoadJob, commands: &mut Commands) {
    let _t = opts_logging::stream_trace::span("tile.surfaces");
    // Surfaces the player's vehicle can stand on: drivable splines (road or rail lanes) and
    // terrain.
    let mut roads = Vec::new();
    for inst in &b.tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            continue;
        };
        if !profile.only_editor
            && (profile
                .paths
                .iter()
                .any(|p| p.path_type == 0 || p.path_type == 2)
                || profile.flat_width() >= SURFACE_MIN_WIDTH_M)
            && !profile.graphical_profiles.is_empty()
        {
            roads.push((inst.clone(), (*profile).clone()));
        }
    }
    let surf: Vec<_> = roads
        .iter()
        .map(|(inst, profile)| {
            let strips = super::super::road_surf_load::strips_for(job, &inst.sli_file, profile);
            (inst.sli_file.clone(), strips)
        })
        .collect();
    job.tile_ground = Some(super::super::GroundIndex::for_tile(
        b.key,
        b.origin,
        b.terrain.clone(),
        &roads,
        job.start_tile,
        job.grid,
    ));
    // The strips first: the ground index attaches them to the samples of the tile's roads.
    super::super::road_surf_load::register(commands, surf);
    super::super::ground_height::register(
        commands,
        b.key,
        b.origin,
        b.terrain.clone(),
        roads,
        job.start_tile,
        job.grid,
    );
    b.phase = Phase::Attachments;
}

pub(super) fn attachments_step(b: &mut TileBuild, job: &mut WorldLoadJob) {
    let _t = opts_logging::stream_trace::span("tile.attachments");
    // Spline attachments become ordinary placements once resolved against their carrier spline.
    b.attached = if b.tile.spline_attachments.is_empty() {
        Vec::new()
    } else {
        let index = job.spline_index();
        let terrain = b.terrain.as_ref();
        expand_attachments(
            &b.tile.spline_attachments,
            &b.tile.splines,
            &index,
            b.key,
            &job.grid,
            |x, y| terrain.map_or(0.0, |t| t.height(x, y)),
        )
    };
    b.phase = Phase::Registries;
}

pub(super) fn registries_step(b: &mut TileBuild, job: &mut WorldLoadJob, commands: &mut Commands) {
    let _t = opts_logging::stream_trace::span("tile.registries");
    let (key, origin, terrain) = (b.key, b.origin, b.terrain.as_ref());
    let lanes = super::super::pedestrians::tile_lanes(job, &b.tile, &b.attached, terrain, origin);
    super::super::pedestrians::register_tile(commands, key, lanes);
    let rail = super::super::rail::tile_rail_lanes(job, &b.tile, &b.attached, terrain, origin);
    super::super::rail::register_tile(commands, key, rail);
    let bikes = super::super::rail::tile_bike_lanes(job, &b.tile, origin);
    super::super::rail::register_bike_tile(commands, key, bikes);
    let road = super::super::traffic::tile_segments(job, &b.tile, terrain, origin);
    super::super::traffic::register_tile(commands, key, road);
    job.scenery.index_tile(&b.tile);
    b.phase = Phase::Objects;
    b.next_object = 0;
    job.placed.clear();
}

/// Spawns the scenery objects from where the build stopped, each one a step of its own. Returns
/// `true` when all are placed.
pub(super) fn objects_step(
    b: &mut TileBuild,
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    budget: &mut FrameBudget,
) -> bool {
    let _t = opts_logging::stream_trace::span("tile.objects");
    let on_tile = b.tile.objects.len();
    let total = on_tile + b.attached.len();
    let mut first = true;
    while b.next_object < total {
        // The step that was granted covers the first object; every further one asks again.
        if !first && !budget.allow() {
            return false;
        }
        first = false;
        let i = b.next_object;
        let (obj, placement, list, index) = if i < on_tile {
            (&b.tile.objects[i], Placement::Tile, "objects", i)
        } else {
            let (o, z) = &b.attached[i - on_tile];
            (o, Placement::OnSpline(*z), "attachments", i - on_tile)
        };
        spawn_object(
            (job, stats, commands),
            (meshes, materials, images),
            (obj, placement, b.terrain.as_ref(), b.key, b.origin),
            (
                &mut b.children,
                &b.tile,
                super::super::trace::Site {
                    list,
                    index,
                    marker: None,
                },
            ),
        );
        b.next_object += 1;
    }
    true
}

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
