//! Objects attached to spline pieces or to other objects: where each one sits.

use super::*;

/// World transform of an object on attachment point `point` of a parent placed at `parent`.
///
/// The point is given in the parent's OMSI axes (x right, y forward, z up; `attach_rot_*` turned
/// as in `[newanim]` frames: `Rx·Ry·Rz`, right-handed); the child's own heading, pitch and roll
/// follow the `[object]` convention (heading clockwise).
pub(super) fn attachment_transform(
    parent: &Transform,
    point: &opts_core::formats::scenery::sco::ScoAttachment,
    heading_deg: f32,
    pitch_deg: f32,
    roll_deg: f32,
) -> Transform {
    let r = point.rotation_deg;
    // OMSI x -> Bevy x, OMSI y (forward) -> Bevy -z, OMSI z (up) -> Bevy y.
    let frame = Quat::from_rotation_x(r.x.to_radians())
        * Quat::from_rotation_z(-r.y.to_radians())
        * Quat::from_rotation_y(r.z.to_radians());
    let local = Transform {
        translation: omsi_to_bevy_pos(point.translation),
        rotation: frame * omsi_euler_to_bevy_quat(-heading_deg, pitch_deg, roll_deg),
        scale: Vec3::ONE,
    };
    parent.mul_transform(local)
}

/// Places the tile's `[attachObj]` children on their parents (objects, spline attachments or other
/// children placed before them, all on this tile). Chains are resolved in rounds; a child whose
/// parent never appears or whose parent has no such attachment point is logged at WARN.
pub(in super::super::super) fn place_attached_objects(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (tile, terrain, key, origin): (
        &opts_core::formats::map::MapTile,
        Option<&TileTerrain>,
        (i32, i32),
        (f32, f32),
    ),
    children: &mut Vec<Entity>,
) {
    let mut pending: Vec<usize> = (0..tile.attached_objects.len()).collect();
    loop {
        let before = pending.len();
        let mut next = Vec::new();
        for i in pending {
            let a = &tile.attached_objects[i];
            let Some((parent, parent_sco)) = job.placed.get(&a.parent_id).cloned() else {
                next.push(i);
                continue;
            };
            let Some(point) = parent_sco.attachments.get(a.attachment) else {
                tracing::warn!(sco = %a.sco_file, parent = a.parent_id, point = a.attachment, points = parent_sco.attachments.len(), tile = ?key, "attached object skipped: the parent has no such attachment point");
                stats.objects_failed += 1;
                continue;
            };
            let transform = attachment_transform(
                &parent,
                point,
                a.heading_deg,
                a.pitch_deg.unwrap_or(0.0),
                a.roll_deg.unwrap_or(0.0),
            );
            // The record as an ordinary object at the resolved spot (tile-local position for
            // whatever samples it, e.g. trees and the scene trace).
            let world = bevy_to_omsi_pos(transform.translation);
            let obj = MapObjectInstance {
                flag: a.flag,
                sco_file: a.sco_file.clone(),
                id_code: a.id_code,
                position: Vec3::new(world.x - origin.0, world.y - origin.1, 0.0),
                heading_deg: a.heading_deg,
                pitch_deg: a.pitch_deg,
                roll_deg: a.roll_deg,
                string_vars: a.string_vars.clone(),
            };
            spawn_object(
                (job, stats, commands),
                (meshes, materials, images),
                (&obj, Placement::OnParent(transform), terrain, key, origin),
                (
                    children,
                    tile,
                    super::super::trace::Site {
                        list: "attached objects",
                        index: i,
                        marker: None,
                    },
                ),
            );
        }
        pending = next;
        if pending.is_empty() || pending.len() == before {
            break;
        }
    }
    for i in pending {
        let a = &tile.attached_objects[i];
        tracing::warn!(sco = %a.sco_file, id = a.id_code, parent = a.parent_id, tile = ?key, "attached object skipped: its parent is not placed on this tile");
        stats.objects_failed += 1;
    }
}
