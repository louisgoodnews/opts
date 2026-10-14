//! Articulated buses and multi-car trams: following segments and the joint variables.
//!
//! The leading unit of a vehicle is the root entity driven by its [`VehicleController`]; every
//! further segment is an entity of its own ([`ArticulatedSegment`]) listed in the root's
//! [`ArticulatedChain`]. The state of the following units lives in the root's
//! [`opts_sim::VehicleSimulation::consist`] (hitch on the leader's rear coupling, rear axle
//! rolling without side slip), which the simulation steps with the chassis; this module only
//! *reads* it each fixed step, hands the poses to the segment entities and writes the joint
//! angles into the script engine of the vehicle as OMSI's models expect them
//! (`docs/src/architecture/articulation.md`):
//!
//! * `articulation_<n>_alpha` — yaw of segment `n + 1` against the unit ahead, **degrees**,
//!   positive when the segment is turned clockwise (the models rotate the bellows dummies by
//!   fractions `0.25 … 1.0` of it with factors in degrees);
//! * `articulation_<n>_beta` / `_gamma` — pitch and roll; `0` for now (flat chain).

use bevy::math::{DVec2, Vec2, Vec3};
use bevy::prelude::*;

use crate::coords::omsi_to_bevy_pos;
use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::VehicleController;

/// A leader position change larger than `max(|v|·dt·2, JUMP_THRESHOLD_M)` in one step is a
/// position jump, not motion: larger than any per-step displacement the simulation produces at
/// any speed, below the 0.5 m steps of the spawn placement.
pub const JUMP_THRESHOLD_M: f32 = 0.3;

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
    /// Mass and axles of the unit for the consist dynamics (`None`: the unit is massless).
    pub load: Option<opts_sim::vehicle::consist::UnitLoad>,
    /// Limits of the pitch `β` in degrees `(min, max)` (`[coupling_front_character]`); the
    /// `articulation_<k>_beta` variable is clamped to them. `None`: unlimited.
    pub beta_limits_deg: Option<(f32, f32)>,
    /// The segment hangs backwards (its model is turned by 180°). Its coupling points and rear
    /// axle are then given mirrored (`-x`, `-y`) and the model is rotated half a turn.
    pub reversed: bool,
    /// Pose after the previous and the last fixed step `(x, y, heading)` in simulation space; the
    /// rendered pose interpolates between them.
    prev_pose: Option<(Vec2, f32)>,
    curr_pose: Option<(Vec2, f32)>,
    /// Height, pitch and roll of the segment on the ground under it, set by the game each frame
    /// (`None`: the segment keeps the root's height and stays level).
    pub ground: Option<SegmentGround>,
    /// Index of the segment's first axle in the consist-wide numbering the model uses
    /// (`Wheel_Rotation_<index>_L`: the head's axles come first, then each trailer's).
    pub first_axle: usize,
    /// The wheels of the segment's own axles (radius, spin), rolling with the segment's travel.
    pub wheels: Vec<SegmentWheel>,
}

/// One axle of a following segment: both wheels roll alike.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentWheel {
    /// Wheel radius (m).
    pub radius: f32,
    /// Accumulated spin (rad), growing while the segment rolls forward along its model.
    pub rotation_rad: f32,
}

/// How a segment lies on the ground (see [`ArticulatedSegment::ground`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SegmentGround {
    /// Height of the segment's origin above the datum (m).
    pub elevation: f32,
    /// Pitch of the segment's body, nose up positive (rad).
    pub pitch: f32,
    /// Roll of the segment's body, right side up positive (rad).
    pub roll: f32,
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
            load: None,
            beta_limits_deg: None,
            reversed: false,
            prev_pose: None,
            curr_pose: None,
            ground: None,
            first_axle: 0,
            wheels: Vec::new(),
        }
    }

    /// `beta` (degrees) limited to the joint's pitch range.
    #[must_use]
    pub fn clamp_beta_deg(&self, beta: f32) -> f32 {
        self.beta_limits_deg
            .map_or(beta, |(lo, hi)| beta.clamp(lo, hi))
    }

    /// Gives the segment its wheels: `first_axle` is the consist-wide index of its first axle and
    /// `radii` the wheel radii of its axles in order.
    pub fn set_wheels(&mut self, first_axle: usize, radii: impl IntoIterator<Item = f32>) {
        self.first_axle = first_axle;
        self.wheels = radii
            .into_iter()
            .map(|radius| SegmentWheel {
                radius: radius.max(0.05),
                rotation_rad: 0.0,
            })
            .collect();
    }

    /// Rolls the wheels by the segment's own travel `from -> to` (simulation space, kinematic
    /// frame, heading `heading` compass rad) and returns the signed distance along the model's
    /// forward direction.
    fn roll(&mut self, from: Vec2, to: Vec2, heading: f32) -> f32 {
        let forward = Vec2::new(heading.sin(), heading.cos());
        let mut d = (to - from).dot(forward);
        if self.reversed {
            d = -d;
        }
        for w in &mut self.wheels {
            w.rotation_rad = (w.rotation_rad + d / w.radius).rem_euclid(std::f32::consts::TAU);
        }
        d
    }

    /// Position and heading of the segment in the kinematic frame (its mirrored frame when
    /// `reversed`): the frame its coupling points and rear axle are given in.
    #[must_use]
    pub fn kinematic_pose(&self) -> Option<(Vec2, f32)> {
        self.curr_pose
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
}

/// The bellows variable `articulation_<k>_beta` (degrees) for a unit pitched `pitch` (rad, nose up
/// positive) behind a unit pitched `leader_pitch`. The bellows dummies turn about the lateral axis
/// by `-beta·factor` (right-handed, the `anim_rot` convention of wheels and `alpha`), so the end of
/// the bellows joined to the follower (behind the joint) only follows it when a nose-up follower
/// gives a **negative** beta (`docs/src/architecture/bellows-census.md`, Sign Verification).
#[must_use]
pub fn joint_beta_deg(leader_pitch: f32, pitch: f32) -> f32 {
    (leader_pitch - pitch).to_degrees()
}

/// The segments behind the root unit, front to back.
#[derive(Component, Debug, Clone, Default)]
pub struct ArticulatedChain {
    /// Segment entities in order.
    pub segments: Vec<Entity>,
}

/// Builds the consist of a root whose simulation has none (or a different number of units than
/// its chain) from the geometry the segments carry. A restored simulation that already holds its
/// consist is left alone.
fn seed_consist(
    sim: &mut opts_sim::VehicleSimulation,
    chain: &[Entity],
    segments: &Query<(&mut ArticulatedSegment, &mut Transform), Without<ArticulatedChain>>,
) {
    sim.clear_consist();
    for &e in chain {
        let Ok((seg, _)) = segments.get(e) else {
            break;
        };
        let (hitch, front) = (seg.hitch_local, seg.front_local);
        sim.attach_trailer(
            DVec2::new(f64::from(hitch.x), f64::from(hitch.y)),
            DVec2::new(f64::from(front.x), f64::from(front.y)),
            f64::from((front.y - seg.rear_axle_y).max(1.0)),
            f64::from(seg.max_alpha_deg),
            seg.load.clone(),
        );
    }
}

/// Reads the consist of every chain after the physics step: copies the poses to the segments,
/// rolls their wheels and writes the joint variables into the scripts. The state itself is the
/// simulation's ([`opts_sim::VehicleSimulation::consist`]).
pub fn articulation_system(
    time: Res<Time>,
    mut last_leader: Local<std::collections::HashMap<Entity, (Vec2, u32)>>,
    mut roots: Query<(
        Entity,
        &mut VehicleController,
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
    for (root, mut controller, root_tf, chain, scripts) in &mut roots {
        let sim = &mut controller.sim;
        if sim.consist.len() != chain.segments.len() {
            seed_consist(sim, &chain.segments, &segments);
        }
        let chassis = &sim.chassis;
        // An explicit `teleport` (spawn, respawn, reset) re-seeded the consist already. A position
        // change the speed cannot explain without one is a jump the caller did not announce
        // (collision push-out, a direct write to the chassis): the consist is re-seeded behind the
        // leader here and the segments' wheels keep their last speed
        // (`docs/src/architecture/chain-assembly.md`).
        let seen = last_leader.insert(root, (chassis.position, sim.teleports));
        let announced = seen.is_none_or(|(_, n)| n != sim.teleports);
        let unannounced = seen.is_some_and(|(prev, _)| {
            let moved = (chassis.position - prev).length();
            moved > (chassis.forward_speed_m_s().abs() * dt as f32 * 2.0).max(JUMP_THRESHOLD_M)
        });
        if unannounced && !announced {
            tracing::debug!(
                ?root,
                "unannounced position jump of a chain leader; consist re-seeded"
            );
            sim.reseed_consist();
        }
        let jumped = announced || unannounced;
        let mut alphas = Vec::with_capacity(chain.segments.len());
        let mut wheel_vars: Vec<(usize, f32, Vec<SegmentWheel>)> = Vec::new();
        for (k, &e) in chain.segments.iter().enumerate() {
            let Ok((mut seg, mut tf)) = segments.get_mut(e) else {
                break;
            };
            let (Some(unit), Some(alpha)) = (sim.consist.get(k), sim.consist_alpha_rad(k)) else {
                break;
            };
            if jumped {
                seg.curr_pose = None;
            }
            let now = (
                Vec2::new(unit.origin.x as f32, unit.origin.y as f32),
                unit.heading as f32,
            );
            let travelled = seg.curr_pose.map_or(0.0, |(p0, _)| {
                // Rolled along the heading half-way through the step (the segment's own motion,
                // not the tractor's speed).
                seg.roll(p0, now.0, now.1)
            });
            if !jumped {
                wheel_vars.push((seg.first_axle, travelled / dt as f32, seg.wheels.clone()));
            }
            seg.prev_pose = seg.curr_pose.or(Some(now));
            seg.curr_pose = Some(now);
            // The placement happens in `articulation_render_system`, interpolated.
            let _ = (&mut tf, root_tf);
            alphas.push(alpha.to_degrees());
        }
        if let Some(scripts) = scripts {
            if let Ok(mut engine) = scripts.engine.lock() {
                // `beta` (pitch) and `gamma` (roll) follow the ground under the segments and are set
                // by the game each frame (`follow_ground_chain`); the kinematics only know yaw.
                for (k, a) in alphas.iter().enumerate() {
                    engine.set(&format!("articulation_{k}_alpha"), *a as f32);
                }
                // The trailers' wheel variables under the consist-wide axle numbers.
                for (first, speed, wheels) in &wheel_vars {
                    for (j, w) in wheels.iter().enumerate() {
                        let i = first + j;
                        let rpm = speed / w.radius * 60.0 / std::f32::consts::TAU;
                        for side in ["L", "R"] {
                            engine.set(&format!("wheel_rotation_{i}_{side}"), w.rotation_rad);
                            engine.set(&format!("axle_steering_{i}_{side}"), 0.0);
                            engine.set(&format!("axle_suspension_{i}_{side}"), 0.0);
                        }
                        engine.set(&format!("wheel_rotationspeed_{i}_l"), rpm);
                        engine.set(&format!("wheel_rotationspeed_{i}_r"), rpm);
                    }
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
            let ground = seg.ground;
            let z = ground.map_or(root_tf.translation.y, |g| g.elevation);
            tf.translation = omsi_to_bevy_pos(Vec3::new(p.x, p.y, z));
            let turn = if seg.reversed {
                std::f32::consts::PI
            } else {
                0.0
            };
            // A reversed segment's model faces the other way: its pitch and roll flip.
            let flip = if seg.reversed { -1.0 } else { 1.0 };
            let (pitch, roll) = ground.map_or((0.0, 0.0), |g| (g.pitch * flip, g.roll * flip));
            tf.rotation = Quat::from_rotation_y(-h - turn)
                * Quat::from_rotation_x(pitch)
                * Quat::from_rotation_z(roll);
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
