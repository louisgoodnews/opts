//! The chain push of the collision pass: a push on a following part is spread along the chain,
//! each joint taking what its limit allows and passing the rest ahead.

use bevy::math::DVec2;

use super::SPREAD_MIN_M;

/// Moves part `part` of the chain (0 = head) by `push` (plan, m). A following part turns about
/// its hitch by what its joint allows (`max_alpha`); the rest goes to the unit ahead, which
/// takes it the same way, and this part is carried rigidly behind that. So a push on a trailer
/// spreads along the chain and ends in the joints and the head.
pub(super) fn push_part(sim: &mut opts_sim::VehicleSimulation, part: usize, push: DVec2) {
    if push.length() <= SPREAD_MIN_M {
        return;
    }
    if part == 0 {
        sim.chassis.position += push.as_vec2();
        return;
    }
    let k = part - 1;
    let (lead, lead_h) = leader_pose(sim, k);
    let unit = &mut sim.consist[k];
    let hitch = unit.hitch(lead, lead_h);
    let target = unit.origin + push;
    // The origin turns about the hitch with the heading: its bearing change is the turn.
    let turn = wrap_pi(bearing(target - hitch) - bearing(unit.origin - hitch));
    let alpha = wrap_pi(unit.heading - lead_h + turn).clamp(-unit.max_alpha, unit.max_alpha);
    let heading = lead_h + alpha;
    let (s, c) = heading.sin_cos();
    let front = DVec2::new(c, -s) * unit.front_local.x + DVec2::new(s, c) * unit.front_local.y;
    unit.heading = heading;
    unit.origin = hitch - front;
    let left = target - unit.origin;
    tracing::debug!(
        part,
        pushed_m = push.length(),
        joint_alpha_deg = alpha.to_degrees(),
        left_m = left.length(),
        "chain push: the joint takes what it allows, the rest goes ahead"
    );
    if left.length() > SPREAD_MIN_M {
        push_part(sim, k, left);
        let (lead, lead_h) = leader_pose(sim, k);
        sim.consist[k].place(lead, lead_h);
    }
}

/// Pose of the unit ahead of consist unit `k` (the chassis for `k = 0`).
fn leader_pose(sim: &opts_sim::VehicleSimulation, k: usize) -> (DVec2, f64) {
    match k.checked_sub(1).and_then(|j| sim.consist.get(j)) {
        Some(ahead) => (ahead.origin, ahead.heading),
        None => {
            let p = sim.chassis.position;
            (
                DVec2::new(f64::from(p.x), f64::from(p.y)),
                f64::from(sim.chassis.heading_rad),
            )
        }
    }
}

/// Compass bearing of a plan vector (0 = +Y, clockwise), the heading of a unit whose forward axis
/// is that vector.
fn bearing(v: DVec2) -> f64 {
    v.x.atan2(v.y)
}

fn wrap_pi(a: f64) -> f64 {
    (a + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}
