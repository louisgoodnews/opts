use bevy::prelude::*;

use crate::mirrors::*;

pub(super) fn cleanup_dead_cameras(
    cameras: &mut Query<(
        Entity,
        &mut MirrorCamera,
        &mut Transform,
        &mut Camera,
        &mut Projection,
    )>,
    panes: &Query<(&GlobalTransform, Option<&InheritedVisibility>), With<MirrorSurface>>,
    vehicles: &Query<&GlobalTransform, With<PlayerVehicle>>,
    quality: &LightQuality,
    virtual_mirrors: &VirtualMirrors,
    commands: &mut Commands,
    stats: &mut MirrorStats,
) -> Vec<Entity> {
    let mut live = Vec::new();
    for (entity, mut info, ..) in &mut *cameras {
        info.members.retain(|m| panes.contains(m.entity));
        let gone = (quality.mirror_count == 0 && !virtual_mirrors.needs_cameras())
            || !vehicles.contains(info.vehicle)
            || info.members.is_empty();
        if gone {
            for m in &info.members {
                if let Ok(mut e) = commands.get_entity(m.entity) {
                    e.insert(MeshMaterial3d(m.original.clone()))
                        .remove::<MirrorSurface>();
                }
            }
            commands.entity(entity).despawn();
        } else {
            live.push(entity);
        }
    }
    stats.cameras = live.len();
    stats.selected = 0;
    stats.rendering = 0;
    live
}
