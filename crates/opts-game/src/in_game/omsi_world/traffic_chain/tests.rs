use super::*;
use std::time::Duration;

/// A lead unit whose rear coupling is 1.5 m behind its origin, one trailing unit whose front
/// joint is 1 m ahead of its origin with the rear axle 7 m behind the hitch.
fn app() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, move_traffic_chains);
    let trailer = app
        .world_mut()
        .spawn((Transform::default(), Visibility::Hidden))
        .id();
    let lead = app
        .world_mut()
        .spawn((
            TrafficVehicle::at(DVec2::new(0.0, 0.0), 0.0, 0.0),
            TrafficChain {
                units: vec![ChainUnit::new(
                    trailer,
                    Vec2::new(0.0, -1.5),
                    Vec2::new(0.0, 1.0),
                    -6.0,
                    52.5,
                    false,
                )],
            },
        ))
        .id();
    (app, lead, trailer)
}

fn tick(app: &mut App, dt: f32) {
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .advance_by(Duration::from_secs_f32(dt));
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(dt));
    app.update();
}

fn trailer_pose(app: &App, trailer: Entity) -> (Vec2, Quat) {
    let t = app.world().get::<Transform>(trailer).expect("transform");
    // Bevy x is OMSI x, Bevy -z is OMSI y.
    (Vec2::new(t.translation.x, -t.translation.z), t.rotation)
}

#[test]
fn the_trailer_hangs_behind_the_lead_and_becomes_visible() {
    let (mut app, _, trailer) = app();
    tick(&mut app, 0.05);
    let (pos, _) = trailer_pose(&app, trailer);
    // Hitch at (0, -1.5); the joint is 1 m ahead of the trailer's origin: origin at y = -2.5.
    assert!(pos.distance(Vec2::new(0.0, -2.5)) < 1e-3, "{pos}");
    assert_eq!(
        *app.world().get::<Visibility>(trailer).expect("visibility"),
        Visibility::Inherited
    );
}

#[test]
fn the_joint_stays_on_the_hitch_and_the_trailer_cuts_the_corner() {
    let (mut app, lead, trailer) = app();
    tick(&mut app, 0.05);
    // The lead drives a quarter circle of radius 15 m at 10 m/s.
    let (radius, speed) = (15.0_f64, 10.0_f64);
    let mut worst_joint_error = 0.0_f32;
    let mut max_yaw_lag = 0.0_f64;
    for k in 1..=40 {
        let dt = 0.05;
        let angle = speed * dt * f64::from(k) / radius;
        let (x, y) = (radius * (1.0 - angle.cos()), radius * angle.sin());
        {
            let mut v = app
                .world_mut()
                .get_mut::<TrafficVehicle>(lead)
                .expect("lead");
            v.origin = DVec2::new(x, y);
            v.heading = angle;
        }
        tick(&mut app, dt as f32);
        let (pos, rot) = trailer_pose(&app, trailer);
        // The trailer's front joint (1 m ahead along its heading) must sit on the hitch.
        let yaw = rot.to_euler(EulerRot::YXZ).0;
        let heading = f64::from(-yaw);
        let joint = pos + Vec2::new(heading.sin() as f32, heading.cos() as f32);
        let hitch = Vec2::new(
            (x + 1.5 * -(angle.sin())) as f32,
            (y - 1.5 * angle.cos()) as f32,
        );
        worst_joint_error = worst_joint_error.max(joint.distance(hitch));
        max_yaw_lag = max_yaw_lag.max(angle - heading);
    }
    assert!(
        worst_joint_error < 0.05,
        "joint off the hitch by {worst_joint_error} m"
    );
    assert!(
        max_yaw_lag > 0.05,
        "a trailer swings behind in a turn: lag {max_yaw_lag} rad"
    );
}
