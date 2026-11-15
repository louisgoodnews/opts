//! Pose of rail units on the track: the frame of a unit, its transform and the tram's joint angles.

use super::*;

/// Pose `d` metres behind the train's head point; a negative `d` is ahead of it, extrapolated along
/// the tangent (the rendered pose runs a fraction of a fixed step ahead of the simulated one).
fn pose_at(train: &Train, net: &PedestrianNet, d: f32) -> Option<(Vec3, f32)> {
    if d >= 0.0 {
        return train.pose_behind(net, d);
    }
    let (p, h) = train.pose_behind(net, 0.0)?;
    Some((p + Vec3::new(h.sin(), h.cos(), 0.0) * -d, h))
}

/// Frame of a unit whose origin is `centre` metres behind the train's track point: its two bogies
/// (`±half_span` about the origin) sit on the track; the origin is the midpoint, the heading the
/// chord between them and the pitch the gradient along it. Without bogie data a 2 m chord is used.
/// Returns the OMSI position, heading and pitch.
pub fn unit_frame(
    train: &Train,
    net: &PedestrianNet,
    centre: f32,
    half_span: f32,
    shift: f32,
) -> Option<(Vec3, f32, f32)> {
    let span = if half_span > 0.0 {
        half_span
    } else {
        MIN_HALF_SPAN
    };
    let front = pose_at(train, net, centre - span - shift)?.0;
    let rear = pose_at(train, net, centre + span - shift)?.0;
    let mid = (front + rear) * 0.5;
    let (dx, dy) = (front.x - rear.x, front.y - rear.y);
    let run = dx.hypot(dy);
    if run > 0.1 {
        Some((mid, dx.atan2(dy), ((front.z - rear.z) / run).atan()))
    } else {
        Some((mid, pose_at(train, net, centre - shift)?.1, 0.0))
    }
}

/// [`unit_frame`] as a transform (OMSI to Bevy), turned half a circle for a reversed unit.
pub fn unit_pose(
    train: &Train,
    net: &PedestrianNet,
    centre: f32,
    half_span: f32,
    reversed: bool,
    shift: f32,
) -> Option<Transform> {
    let (mid, heading, pitch) = unit_frame(train, net, centre, half_span, shift)?;
    let turn = if reversed { std::f32::consts::PI } else { 0.0 };
    Some(Transform {
        translation: omsi_to_bevy_pos(mid),
        rotation: Quat::from_rotation_y(-heading)
            * Quat::from_rotation_x(pitch)
            * Quat::from_rotation_y(turn),
        scale: Vec3::ONE,
    })
}

/// Places the player's tram (head and following units) on the track, and writes the joint angles
/// the models' bellows read.
pub fn rail_player_pose(
    fixed: Res<Time<Fixed>>,
    net: Res<RailNetwork>,
    mut players: Query<(
        &RailPlayer,
        &mut Transform,
        Option<&VehicleComposition>,
        Option<&opts_bevy::VehicleScripts>,
    )>,
    mut units: Query<&mut Transform, Without<RailPlayer>>,
) {
    for (rail, mut tf, layout, scripts) in &mut players {
        let Some(train) = rail.train.as_ref() else {
            continue;
        };
        let head_span = rail.span;
        // The simulation steps at a fixed rate: the drawn train runs the elapsed part of the next
        // step ahead, so its motion is smooth at any frame rate.
        let shift = rail.speed * fixed.overstep().as_secs_f32();
        if let Some(t) = unit_pose(train, &net.0, head_span, head_span, false, shift) {
            *tf = t;
        }
        let Some(layout) = layout else { continue };
        let mut previous = tf.rotation.to_euler(EulerRot::YXZ).0;
        let mut alphas = Vec::with_capacity(layout.followers.len());
        for part in &layout.followers {
            let (e, reversed) = (part.entity, part.reversed);
            let Some(pose) = unit_pose(
                train,
                &net.0,
                part.behind + head_span,
                part.half_span,
                reversed,
                shift,
            ) else {
                continue;
            };
            let yaw = pose.rotation.to_euler(EulerRot::YXZ).0;
            let own = if reversed {
                yaw + std::f32::consts::PI
            } else {
                yaw
            };
            let d = (own - previous + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            // Bevy yaw is counter-clockwise; the models expect clockwise degrees.
            alphas.push(-d.to_degrees());
            previous = own;
            if let Ok(mut u) = units.get_mut(e) {
                *u = pose;
            }
        }
        if let Some(scripts) = scripts {
            if let Ok(mut engine) = scripts.engine.lock() {
                for (k, a) in alphas.iter().enumerate() {
                    engine.set(&format!("articulation_{k}_alpha"), *a);
                    engine.set(&format!("articulation_{k}_beta"), 0.0);
                    engine.set(&format!("articulation_{k}_gamma"), 0.0);
                }
            }
        }
    }
}
