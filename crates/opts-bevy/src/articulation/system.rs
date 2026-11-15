//! The articulation systems: fixed-step reading of the consist and the per-frame placement.

use bevy::math::{Vec2, Vec3};
use bevy::prelude::*;

use crate::coords::omsi_to_bevy_pos;
use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::VehicleController;

use super::chain::{detect_position_jump, seed_consist, ArticulatedChain};
use super::scripts::apply_articulation_scripts;
use super::segment::{ArticulatedSegment, SegmentGround, SegmentWheel};

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
        let jumped = detect_position_jump(sim, root, &mut last_leader, dt);
        let mut alphas = Vec::with_capacity(chain.segments.len());
        let mut joints: Vec<(usize, f32, f32)> = Vec::new();
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
                seg.curr_ground = None;
            }
            // Pitch, roll and height from the simulation's fixed-step ground following.
            seg.prev_ground = seg.curr_ground;
            seg.curr_ground = sim.consist_body(k).map(|b| SegmentGround {
                elevation: b.elevation,
                pitch: b.pitch,
                roll: b.roll,
            });
            if seg.prev_ground.is_none() {
                seg.prev_ground = seg.curr_ground;
            }
            if let Some((beta, gamma)) = sim.joint_angles_deg(k) {
                // The joint cannot pitch beyond `[coupling_front_character]` min/max beta.
                joints.push((k, seg.clamp_beta_deg(beta), gamma));
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
        // The trailers' axles have no body model: they rest at the head's mean travel.
        let rest = sim.body.as_ref().map_or(0.0, |b| b.rest_travel_m());
        if let Some(scripts) = scripts {
            apply_articulation_scripts(scripts, &joints, &alphas, &wheel_vars, rest);
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
            // Height, pitch and roll interpolate between the last two fixed steps like the pose.
            let ground = match (seg.prev_ground, seg.curr_ground) {
                (Some(a), Some(c)) => Some(SegmentGround {
                    elevation: a.elevation + (c.elevation - a.elevation) * alpha,
                    pitch: a.pitch + (c.pitch - a.pitch) * alpha,
                    roll: a.roll + (c.roll - a.roll) * alpha,
                }),
                (_, c) => c,
            };
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
