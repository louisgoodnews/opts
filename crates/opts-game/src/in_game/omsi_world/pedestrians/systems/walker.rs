//! The pedestrian walkers: spawning a standing human on a path, and walking, posing and removing
//! them each frame.

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
pub fn move_pedestrians(
    time: Res<Time>,
    mut commands: Commands,
    net: Res<PedestrianNetwork>,
    mut pool: ResMut<PedestrianPool>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut peds: Query<(Entity, &mut Pedestrian, &Human)>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs().min(0.1);
    let cam = camera.single().ok().map(|c| c.translation());
    let mut rng = pool.rng ^ 0x5bd1_e995;
    for (entity, mut ped, human) in &mut peds {
        let outcome = ped.walker.advance(dt, &net.0, &mut rng);
        let Some((pos, heading)) = ped.walker.pose(&net.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        let bevy_pos = omsi_to_bevy_pos(pos);
        let far = cam.is_some_and(|c| c.distance(bevy_pos) > DESPAWN_M);
        if outcome == Advance::Finished || far {
            commands.entity(entity).despawn();
            continue;
        }
        if let Ok(mut t) = transforms.get_mut(entity) {
            t.translation = bevy_pos;
            t.rotation = omsi_euler_to_bevy_quat(-heading.to_degrees(), 0.0, 0.0);
        }
        // Gait.
        ped.walker.phase = advance_phase(human, ped.walker.phase, ped.walker.speed, dt);
        apply_gait(human, ped.leg_m, ped.walker.phase, &mut transforms);
    }
    pool.rng = rng;
}
