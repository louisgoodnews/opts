//! The scenery object step of a [`TileBuild`](super::super::TileBuild), one object per step.

use super::super::super::tile::{spawn_object, Placement};
use super::super::*;

/// Spawns the scenery objects from where the build stopped, each one a step of its own. Returns
/// `true` when all are placed.
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
