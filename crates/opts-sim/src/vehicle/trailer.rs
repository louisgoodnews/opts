//! Following segments of articulated buses and multi-car trams.
//!
//! Every segment is a vehicle of its own whose front coupling (`[coupling_front]`) sits on the
//! rear coupling (`[coupling_back]`) of the unit ahead. The segment's rear axle rolls without
//! sliding sideways, which fixes how it turns: with `H` the hitch point, `L` the distance from
//! the hitch to the segment's rearmost axle and `r` the segment's right-hand unit vector,
//!
//! ```text
//! dψ/dt = (v_H · r) / L
//! ```
//!
//! (the sideways speed of the hitch swings the segment around its axle). The model is purely
//! kinematic, so a segment off-tracks inwards on curves like the real thing and the angle
//! between the units (`α`, positive when the segment is turned clockwise against the leader) is
//! limited by the joint (`[coupling_front_character]`, typically ±52.5°).
//!
//! Conventions: world `x` east, `y` north, headings compass-style (clockwise from north).

use glam::DVec2;
use serde::{Deserialize, Serialize};

/// Largest integration step (s) of [`TrailerKinematics::step`].
const MAX_SUBSTEP_S: f64 = 0.05;

/// A following segment. The whole state (including the previous hitch position the hitch
/// velocity is derived from) is serialisable, so a saved consist resumes without a seam.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrailerKinematics {
    /// Rear coupling of the leading unit `(x, y)` in its own frame.
    pub hitch_local: DVec2,
    /// Front coupling of this segment `(x, y)` in its own frame.
    pub front_local: DVec2,
    /// Distance from the hitch to the segment's rear axle (m).
    pub axle_distance: f64,
    /// Joint limit on `|α|` in radians.
    pub max_alpha: f64,
    /// Current heading of the segment (compass radians).
    pub heading: f64,
    /// World position of the segment's origin.
    pub origin: DVec2,
    prev_hitch: Option<DVec2>,
}

/// State of a segment after a step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrailerState {
    /// World position of the segment's origin.
    pub origin: DVec2,
    /// Heading of the segment (compass radians).
    pub heading: f64,
    /// Yaw angle against the leader in radians (positive = segment turned clockwise).
    pub alpha: f64,
}

fn fwd(h: f64) -> DVec2 {
    DVec2::new(h.sin(), h.cos())
}

fn right(h: f64) -> DVec2 {
    DVec2::new(h.cos(), -h.sin())
}

fn wrap(a: f64) -> f64 {
    (a + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

impl TrailerKinematics {
    /// A segment hanging straight behind its leader at `leader_pos`/`leader_heading`.
    #[must_use]
    pub fn new(
        hitch_local: DVec2,
        front_local: DVec2,
        axle_distance: f64,
        max_alpha_deg: f64,
        leader_pos: DVec2,
        leader_heading: f64,
    ) -> Self {
        let mut t = Self {
            hitch_local,
            front_local,
            axle_distance: axle_distance.max(1.0),
            max_alpha: max_alpha_deg.to_radians().abs().clamp(0.1, 3.0),
            heading: leader_heading,
            origin: leader_pos,
            prev_hitch: None,
        };
        t.reseed(leader_pos, leader_heading);
        t
    }

    /// Hangs the segment straight behind a leader at `leader_pos`/`leader_heading` with no
    /// velocity (joint angle 0): the response to a position jump (spawn, respawn, reset).
    pub fn reseed(&mut self, leader_pos: DVec2, leader_heading: f64) {
        self.heading = leader_heading;
        self.prev_hitch = None;
        self.origin = self.hitch(leader_pos, leader_heading) - self.front_offset(leader_heading);
    }

    /// Yaw angle against a leader heading `leader_heading` in radians (positive = segment turned
    /// clockwise), wrapped to `(-π, π]`.
    #[must_use]
    pub fn alpha_against(&self, leader_heading: f64) -> f64 {
        wrap(self.heading - leader_heading)
    }

    /// World position of the leader's rear coupling.
    #[must_use]
    pub fn hitch(&self, leader_pos: DVec2, leader_heading: f64) -> DVec2 {
        leader_pos
            + right(leader_heading) * self.hitch_local.x
            + fwd(leader_heading) * self.hitch_local.y
    }

    fn front_offset(&self, heading: f64) -> DVec2 {
        right(heading) * self.front_local.x + fwd(heading) * self.front_local.y
    }

    /// Advances the segment by `dt` seconds while the leader moves to `leader_pos` heading
    /// `leader_heading` (its state at the end of the step).
    pub fn step(&mut self, leader_pos: DVec2, leader_heading: f64, dt: f64) -> TrailerState {
        let hitch = self.hitch(leader_pos, leader_heading);
        let velocity = match self.prev_hitch {
            Some(prev) if dt > 1e-6 => (hitch - prev) / dt,
            _ => DVec2::ZERO,
        };
        self.prev_hitch = Some(hitch);
        let n = (dt / MAX_SUBSTEP_S).ceil().max(1.0) as usize;
        let h = dt / n as f64;
        for _ in 0..n {
            let rate = velocity.dot(right(self.heading)) / self.axle_distance;
            self.heading += rate * h;
        }
        let alpha = wrap(self.heading - leader_heading).clamp(-self.max_alpha, self.max_alpha);
        self.heading = leader_heading + alpha;
        self.origin = hitch - self.front_offset(self.heading);
        TrailerState {
            origin: self.origin,
            heading: self.heading,
            alpha,
        }
    }
}

#[cfg(test)]
mod tests;
