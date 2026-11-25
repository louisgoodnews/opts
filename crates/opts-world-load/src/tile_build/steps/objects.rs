//! The scenery object step of a [`TileBuild`](super::super::TileBuild), one object per step.

use super::super::super::tile::{spawn_object, Placement};
use super::super::*;
use opts_stream_budget::order::build_order;

/// Spawns the scenery objects from where the build stopped, each one a step of its own. Returns
/// `true` when all are placed. The order is nearest-first while the player is slow (M-0114).
pub(in super::super) fn objects_step(
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
    if b.order.is_none() {
        let points = scene_points(b);
        b.order = Some(build_order(&points, job.stream_focus));
    }
    let mut first = true;
    while b.next_object < total {
        // The step that was granted covers the first object; every further one asks again.
        if !first && !budget.allow() {
            return false;
        }
        first = false;
        let i = b.order.as_ref().map_or(b.next_object, |o| o[b.next_object]);
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
                super::super::super::trace::Site {
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

/// Scene-frame ground positions of the tile's objects, then its attachments (the order of
/// `TileBuild::next_object`).
fn scene_points(b: &TileBuild) -> Vec<(f32, f32)> {
    let at = |p: bevy::math::Vec3| (b.origin.0 + p.x, b.origin.1 + p.y);
    b.tile
        .objects
        .iter()
        .map(|o| at(o.position))
        .chain(b.attached.iter().map(|(o, _)| at(o.position)))
        .collect()
}
