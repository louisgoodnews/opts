//! The pedestrian walkers: spawning a standing human on a path, and walking, posing and removing
//! them each frame.

use std::collections::HashMap;

use opts_pedestrian_rules::lod::{detail_at, Detail};
use opts_pedestrian_rules::separation::separated_speeds;

use super::super::*;

/// Spawns the character as a standing human on the walker's path and records it in the scene trace.
pub(super) fn spawn_walker(
    commands: &mut Commands,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    net: &PedestrianNet,
    v: &Variant,
    walker: Walker,
) {
    let leg_m = rest_pivots(&v.profile)[0].y.max(0.3);
    let root = spawn_human(
        commands,
        bindposes,
        &v.profile,
        &v.parts,
        &v.name,
        HumanPose::Standing,
    );
    if opts_bevy::scene_trace::enabled() {
        let frame = super::super::super::trace::human_frame(&v.name);
        let tf = Transform::from_translation(
            walker
                .pose(net)
                .map(|(p, _)| omsi_to_bevy_pos(p))
                .unwrap_or_default(),
        );
        super::super::super::trace::spawned(
            opts_bevy::scene_trace::kind::HUMAN,
            &v.name,
            &v.name,
            (root, &tf),
            &frame,
            "ok",
            serde_json::json!({}),
        );
    }
    commands.entity(root).insert((
        Pedestrian { walker, leg_m },
        Visibility::default(),
        InGameEntity,
    ));
}

/// Walks the pedestrians, poses them and removes finished or distant ones.
///
/// A walker behind another walker keeps no faster than it ([`separated_speeds`]); a walker about
/// to step onto a crossing whose light is at stop holds ([`PedestrianCrossings`]); a walker
/// beyond the full detail distance keeps its last pose, without the gait ([`detail_at`]).
pub fn move_pedestrians(
    time: Res<Time>,
    mut commands: Commands,
    net: Res<PedestrianNetwork>,
    crossings: Res<PedestrianCrossings>,
    mut pool: ResMut<PedestrianPool>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut peds: Query<(Entity, &mut Pedestrian, &Human)>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs().min(0.1);
    let cam = camera.single().ok().map(|c| c.translation());
    let mut rng = pool.rng ^ 0x5bd1_e995;
    // The speeds are worked out from the walkers as they stand at the start of the frame.
    let snapshot: Vec<(Entity, Walker)> = peds.iter().map(|(e, p, _)| (e, p.walker)).collect();
    let walkers: Vec<Walker> = snapshot.iter().map(|(_, w)| *w).collect();
    let speeds: HashMap<Entity, f32> = snapshot
        .iter()
        .map(|(e, _)| *e)
        .zip(separated_speeds(&walkers, &net.0))
        .collect();
    for (entity, mut ped, human) in &mut peds {
        // The walking speed for this frame only; the walker keeps its own speed.
        let base = ped.walker;
        let mut walker = base;
        walker.speed = speeds.get(&entity).copied().unwrap_or(base.speed);
        let held = crossings.gates.holds(&net.0, &walker, dt);
        let outcome = if held {
            walker.speed = 0.0;
            Advance::Moving
        } else {
            walker.advance(dt, &net.0, &mut rng)
        };
        let gait_speed = walker.speed;
        ped.walker = Walker {
            speed: base.speed,
            ..walker
        };
        let Some((pos, heading)) = ped.walker.pose(&net.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        let bevy_pos = omsi_to_bevy_pos(pos);
        let distance = cam.map_or(f32::NAN, |c| c.distance(bevy_pos));
        if outcome == Advance::Finished || distance > DESPAWN_M {
            commands.entity(entity).despawn();
            continue;
        }
        if let Ok(mut t) = transforms.get_mut(entity) {
            t.translation = bevy_pos;
            t.rotation = omsi_euler_to_bevy_quat(-heading.to_degrees(), 0.0, 0.0);
        }
        // Gait, unless the walker is drawn frozen (a missing camera counts as near).
        if detail_at(distance) == Detail::Full {
            ped.walker.phase = advance_phase(human, ped.walker.phase, gait_speed, dt);
            apply_gait(human, ped.leg_m, ped.walker.phase, &mut transforms);
        }
    }
    pool.rng = rng;
}
