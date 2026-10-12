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
    /// The segment hangs backwards (its model is turned by 180°). Its coupling points and rear
    /// axle are then given mirrored (`-x`, `-y`) and the model is rotated half a turn.
    pub reversed: bool,
    /// The kinematic state, created on the first step from the leader's pose.
    state: Option<TrailerKinematics>,
    /// Pose after the previous and the last fixed step `(x, y, heading)` in simulation space; the
    /// rendered pose interpolates between them.
    prev_pose: Option<(Vec2, f32)>,
    curr_pose: Option<(Vec2, f32)>,
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
            reversed: false,
            state: None,
            prev_pose: None,
            curr_pose: None,
        }
    }

    /// Position and heading (compass, rad) of the segment's own frame after the last fixed step
    /// (a reversed segment's frame is turned half a circle against the kinematic one).
    #[must_use]
    pub fn simulated_pose(&self) -> Option<(Vec2, f32)> {
        let (p, h) = self.curr_pose?;
        Some((
            p,
            if self.reversed {
                h + std::f32::consts::PI
            } else {
                h
            },
        ))
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
            let now = (
                Vec2::new(st.origin.x as f32, st.origin.y as f32),
                st.heading as f32,
            );
            seg.prev_pose = seg.curr_pose.or(Some(now));
            seg.curr_pose = Some(now);
            // The placement happens in `articulation_render_system`, interpolated.
            let _ = (&mut tf, root_tf);
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

/// Places the segments every frame between their last two fixed-step poses (smooth at any frame
/// rate), at the height of the root unit (the chain is flat).
pub fn articulation_render_system(
    fixed: Res<Time<Fixed>>,
    roots: Query<(&Transform, &ArticulatedChain)>,
    mut segments: Query<(&ArticulatedSegment, &mut Transform), Without<ArticulatedChain>>,
) {
    let alpha = fixed.overstep_fraction().clamp(0.0, 1.0);
    for (root_tf, chain) in &roots {
        for &e in &chain.segments {
            let Ok((seg, mut tf)) = segments.get_mut(e) else {
                continue;
            };
            let (Some((p0, h0)), Some((p1, h1))) = (seg.prev_pose, seg.curr_pose) else {
                continue;
            };
            let dh = (h1 - h0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            let (p, h) = (p0.lerp(p1, alpha), h0 + dh * alpha);
            tf.translation = omsi_to_bevy_pos(Vec3::new(p.x, p.y, root_tf.translation.y));
            let turn = if seg.reversed {
                std::f32::consts::PI
            } else {
                0.0
            };
            tf.rotation = Quat::from_rotation_y(-h - turn);
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
                .after(crate::RailSet)
                .before(crate::vehicle_script::vehicle_script_system),
        )
        // After the ground follower of the game (Update) has set the root's height.
        .add_systems(
            PostUpdate,
            articulation_render_system.before(TransformSystems::Propagate),
        );
    }
}
