//! Articulated buses and multi-car trams: following segments and the joint variables.
//!
//! The leading unit of a vehicle is the root entity driven by its [`VehicleController`]; every
//! further segment is an entity of its own ([`ArticulatedSegment`]) listed in the root's
//! [`ArticulatedChain`]. Each fixed step the segments are advanced one behind the other with
//! [`opts_sim::TrailerKinematics`] (hitch on the leader's rear coupling, rear axle rolling
//! without side slip) and placed in the world. The joint angles are written into the script
//! engine of the vehicle as OMSI's models expect them
//! (`docs/src/architecture/articulation.md`):
//!
//! * `articulation_<n>_alpha` — yaw of segment `n + 1` against the unit ahead, **degrees**,
//!   positive when the segment is turned clockwise (the models rotate the bellows dummies by
//!   fractions `0.25 … 1.0` of it with factors in degrees);
//! * `articulation_<n>_beta` / `_gamma` — pitch and roll; `0` for now (flat chain).

use bevy::math::{DVec2, Vec2, Vec3};
use bevy::prelude::*;
use opts_sim::TrailerKinematics;

use crate::coords::omsi_to_bevy_pos;
use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::VehicleController;

/// Geometry of a following segment, taken from the `.bus` files of the two units.
#[derive(Component, Debug, Clone)]
pub struct ArticulatedSegment {
    /// Rear coupling `(x, y)` of the unit ahead.
    pub hitch_local: Vec2,
    /// Front coupling `(x, y)` of this segment.
    pub front_local: Vec2,
    /// Position `y` of the segment's rearmost axle in its own frame.
    pub rear_axle_y: f32,
    /// Joint limit on `|α|` in degrees (`[coupling_front_character]`).
    pub max_alpha_deg: f32,
    /// The kinematic state, created on the first step from the leader's pose.
    state: Option<TrailerKinematics>,
}

impl ArticulatedSegment {
    /// Describes a segment; it hangs straight behind its leader until the first step.
    #[must_use]
    pub fn new(hitch_local: Vec2, front_local: Vec2, rear_axle_y: f32, max_alpha_deg: f32) -> Self {
        Self {
            hitch_local,
            front_local,
            rear_axle_y,
            max_alpha_deg,
            state: None,
        }
    }

    /// The yaw angle against the leader in degrees, once the segment has been stepped.
    #[must_use]
    pub fn alpha_deg(&self, leader_heading: f64) -> Option<f64> {
        let t = self.state.as_ref()?;
        let d = (t.heading - leader_heading + std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        Some(d.to_degrees())
    }
}

/// The segments behind the root unit, front to back.
#[derive(Component, Debug, Clone, Default)]
pub struct ArticulatedChain {
    /// Segment entities in order.
    pub segments: Vec<Entity>,
}

/// Advances all chains by one fixed step.
pub fn articulation_system(
    time: Res<Time>,
    roots: Query<(
        &VehicleController,
        &Transform,
        &ArticulatedChain,
        Option<&VehicleScripts>,
    )>,
    mut segments: Query<(&mut ArticulatedSegment, &mut Transform), Without<ArticulatedChain>>,
) {
    let dt = f64::from(time.delta_secs());
    if dt <= 0.0 {
        return;
    }
    for (controller, root_tf, chain, scripts) in &roots {
        let chassis = &controller.sim.chassis;
        let mut leader = (
            DVec2::new(f64::from(chassis.position.x), f64::from(chassis.position.y)),
            f64::from(chassis.heading_rad),
        );
        let mut alphas = Vec::with_capacity(chain.segments.len());
        for &e in &chain.segments {
            let Ok((mut seg, mut tf)) = segments.get_mut(e) else {
                break;
            };
            let (hitch, front) = (seg.hitch_local, seg.front_local);
            let axle_distance = f64::from((front.y - seg.rear_axle_y).max(1.0));
            let max_alpha = f64::from(seg.max_alpha_deg);
            let kin = seg.state.get_or_insert_with(|| {
                TrailerKinematics::new(
                    DVec2::new(f64::from(hitch.x), f64::from(hitch.y)),
                    DVec2::new(f64::from(front.x), f64::from(front.y)),
                    axle_distance,
                    max_alpha,
                    leader.0,
                    leader.1,
                )
            });
            let st = kin.step(leader.0, leader.1, dt);
            // The chain is flat: the segment keeps the height of the root unit.
            tf.translation = omsi_to_bevy_pos(Vec3::new(
                st.origin.x as f32,
                st.origin.y as f32,
                root_tf.translation.y,
            ));
            tf.rotation = Quat::from_rotation_y(-(st.heading as f32));
            alphas.push(st.alpha.to_degrees());
            leader = (st.origin, st.heading);
        }
        if let Some(scripts) = scripts {
            if let Ok(mut engine) = scripts.engine.lock() {
                for (k, a) in alphas.iter().enumerate() {
                    engine.set(&format!("articulation_{k}_alpha"), *a as f32);
                    engine.set(&format!("articulation_{k}_beta"), 0.0);
                    engine.set(&format!("articulation_{k}_gamma"), 0.0);
                }
            }
        }
    }
}

/// Registers [`articulation_system`] between the physics step and the scripts.
#[derive(Default)]
pub struct ArticulationPlugin;

impl Plugin for ArticulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            articulation_system
                .after(crate::vehicle_sim::vehicle_physics_step_system)
                .before(crate::vehicle_script::vehicle_script_system),
        );
    }
}
