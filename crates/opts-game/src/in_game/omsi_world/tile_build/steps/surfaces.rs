//! Surface and registry steps of a [`TileBuild`](super::super::TileBuild): the drivable surfaces and
//! ground index, the spline attachments, and the per-tile registries.

use crate::in_game::attachments::expand_attachments;

use super::super::super::tile::SURFACE_MIN_WIDTH_M;
use super::super::*;

pub(in super::super) fn surfaces_step(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    commands: &mut Commands,
) {
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
            let strips =
                super::super::super::road_surf_load::strips_for(job, &inst.sli_file, profile);
            (inst.sli_file.clone(), strips)
        })
        .collect();
    job.tile_ground = Some(super::super::super::GroundIndex::for_tile(
        b.key,
        b.origin,
        b.terrain.clone(),
        &roads,
        job.start_tile,
        job.grid,
    ));
    // The strips first: the ground index attaches them to the samples of the tile's roads.
    super::super::super::road_surf_load::register(commands, surf);
    super::super::super::ground_height::register(
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

pub(in super::super) fn attachments_step(b: &mut TileBuild, job: &mut WorldLoadJob) {
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

pub(in super::super) fn registries_step(
    b: &mut TileBuild,
    job: &mut WorldLoadJob,
    commands: &mut Commands,
) {
    let _t = opts_logging::stream_trace::span("tile.registries");
    let (key, origin, terrain) = (b.key, b.origin, b.terrain.as_ref());
    let lanes =
        super::super::super::pedestrians::tile_lanes(job, &b.tile, &b.attached, terrain, origin);
    super::super::super::pedestrians::register_tile(commands, key, lanes);
    let rail =
        super::super::super::rail::tile_rail_lanes(job, &b.tile, &b.attached, terrain, origin);
    super::super::super::rail::register_tile(commands, key, rail);
    let bikes = super::super::super::rail::tile_bike_lanes(job, &b.tile, origin);
    super::super::super::rail::register_bike_tile(commands, key, bikes);
    let road = super::super::super::traffic::tile_segments(job, &b.tile, terrain, origin);
    super::super::super::traffic::register_tile(commands, key, road);
    job.scenery.index_tile(&b.tile);
    b.phase = Phase::Objects;
    b.next_object = 0;
    job.placed.clear();
}
