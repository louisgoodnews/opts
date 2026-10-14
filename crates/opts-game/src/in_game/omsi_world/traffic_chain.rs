//! Trailing units of coupled AI vehicles (articulated buses, lorries with trailers): each unit
//! follows the unit ahead through the same kinematic joint the player's consist uses
//! ([`TrailerKinematics`]), pulled behind the lead vehicle that the lane model drives.

use bevy::math::{DVec2, Vec2};
use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_sim::vehicle::trailer::TrailerKinematics;

use super::traffic::TrafficVehicle;

/// One trailing unit.
pub struct ChainUnit {
    entity: Entity,
    hitch: DVec2,
    joint: DVec2,
    axle_distance: f64,
    max_alpha_deg: f64,
    reversed: bool,
    kin: Option<TrailerKinematics>,
}

impl ChainUnit {
    /// A unit whose front `joint` sits on its predecessor's rear coupling `hitch` (both in
    /// their own vehicle frames), with its rearmost axle at `rear_axle_y`.
    #[must_use]
    pub fn new(
        entity: Entity,
        hitch: Vec2,
        joint: Vec2,
        rear_axle_y: f32,
        max_alpha_deg: f32,
        reversed: bool,
    ) -> Self {
        Self {
            entity,
            hitch: hitch.as_dvec2(),
            joint: joint.as_dvec2(),
            axle_distance: f64::from((joint.y - rear_axle_y).max(1.0)),
            max_alpha_deg: f64::from(max_alpha_deg),
            reversed,
            kin: None,
        }
    }
}

/// The trailing units of a lead vehicle, in order.
#[derive(Component)]
pub struct TrafficChain {
    pub(super) units: Vec<ChainUnit>,
}

/// Despawns a lead vehicle together with its trailing units.
pub(super) fn despawn_chain(commands: &mut Commands, lead: Entity, chain: Option<&TrafficChain>) {
    for unit in chain.into_iter().flat_map(|c| &c.units) {
        commands.entity(unit.entity).despawn();
    }
    commands.entity(lead).despawn();
}

/// Places the trailing units behind their lead vehicle every frame.
pub fn move_traffic_chains(
    time: Res<Time>,
    mut leads: Query<(&TrafficVehicle, &mut TrafficChain)>,
    mut parts: Query<(&mut Transform, &mut Visibility), Without<TrafficVehicle>>,
) {
    let dt = f64::from(time.delta_secs().min(0.2));
    for (lead, mut chain) in &mut leads {
        let mut leader = (lead.origin, lead.heading);
        for unit in &mut chain.units {
            let (hitch, joint, axle, alpha) = (
                unit.hitch,
                unit.joint,
                unit.axle_distance,
                unit.max_alpha_deg,
            );
            let kin = unit.kin.get_or_insert_with(|| {
                TrailerKinematics::new(hitch, joint, axle, alpha, leader.0, leader.1)
            });
            let state = kin.step(leader.0, leader.1, dt);
            if let Ok((mut transform, mut visibility)) = parts.get_mut(unit.entity) {
                transform.translation = omsi_to_bevy_pos(Vec3::new(
                    state.origin.x as f32,
                    state.origin.y as f32,
                    lead.height,
                ));
                let turn = if unit.reversed { 180.0 } else { 0.0 };
                transform.rotation =
                    omsi_euler_to_bevy_quat(-(state.heading.to_degrees() as f32 + turn), 0.0, 0.0);
                if *visibility != Visibility::Inherited {
                    *visibility = Visibility::Inherited;
                }
            }
            leader = (state.origin, state.heading);
        }
    }
}

#[cfg(test)]
mod tests {
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
}
