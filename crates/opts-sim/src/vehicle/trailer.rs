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

/// Largest integration step (s) of [`TrailerKinematics::step`].
const MAX_SUBSTEP_S: f64 = 0.05;

/// A following segment.
#[derive(Debug, Clone, PartialEq)]
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
        t.origin = t.hitch(leader_pos, leader_heading) - t.front_offset(leader_heading);
        t
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
mod tests {
    use super::*;

    /// A leader whose origin sits at the rear axle, hitch 1.5 m behind it; segment front coupling
    /// 1 m ahead of the segment origin, rear axle 8 m behind the hitch.
    fn joint(leader_pos: DVec2, heading: f64) -> TrailerKinematics {
        TrailerKinematics::new(
            DVec2::new(0.0, -1.5),
            DVec2::new(0.0, 1.0),
            8.0,
            52.5,
            leader_pos,
            heading,
        )
    }

    /// Drives the leader along a circle of `radius` (positive = clockwise) at `speed`.
    fn drive_circle(
        t: &mut TrailerKinematics,
        radius: f64,
        speed: f64,
        seconds: f64,
    ) -> Vec<TrailerState> {
        let dt = 0.02;
        let omega = speed / radius;
        let centre = DVec2::new(radius, 0.0); // leader starts at the origin heading north
        let mut out = Vec::new();
        let mut time = 0.0;
        while time < seconds {
            time += dt;
            let h = omega * time;
            // Position on the circle around `centre`, heading tangent (clockwise for radius > 0).
            let pos = centre + DVec2::new(-radius * h.cos(), radius * h.sin());
            out.push(t.step(pos, h, dt));
        }
        out
    }

    #[test]
    fn straight_driving_keeps_the_segment_in_line_and_hitched() {
        let mut t = joint(DVec2::ZERO, 0.0);
        for k in 1..=200 {
            let pos = DVec2::new(0.0, f64::from(k) * 0.1);
            let s = t.step(pos, 0.0, 0.01);
            assert!(s.alpha.abs() < 1e-9);
            // The segment's front coupling is on the hitch.
            let hitch = t.hitch(pos, 0.0);
            assert!((s.origin + t.front_offset(s.heading) - hitch).length() < 1e-9);
        }
    }

    #[test]
    fn a_curve_turns_the_segment_against_the_leader_and_it_off_tracks_inwards() {
        let mut t = joint(DVec2::ZERO, 0.0);
        let states = drive_circle(&mut t, 20.0, 5.0, 40.0);
        let last = states[states.len() - 1];
        // Steady state: the angle stops changing …
        let before = states[states.len() - 500].alpha;
        assert!(
            (last.alpha - before).abs() < 0.01,
            "α still moving: {before} → {}",
            last.alpha
        );
        // … the leader turns clockwise, so the segment lags: α < 0 (anticlockwise against it).
        assert!(last.alpha < -0.2 && last.alpha > -1.0, "α = {}", last.alpha);
        // Off-tracking: the segment's rear axle runs on a smaller circle than the hitch.
        let centre = DVec2::new(20.0, 0.0);
        let hitch_r = (t.hitch(
            centre
                + DVec2::new(
                    -20.0 * (5.0f64 / 20.0 * 40.0).cos(),
                    20.0 * (5.0f64 / 20.0 * 40.0).sin(),
                ),
            5.0 / 20.0 * 40.0,
        ) - centre)
            .length();
        let axle = last.origin + (right(last.heading) * 0.0 + fwd(last.heading) * (1.0 - 8.0));
        assert!(
            (axle - centre).length() < hitch_r,
            "rear axle {} vs hitch {hitch_r}",
            (axle - centre).length()
        );
    }

    #[test]
    fn mirrored_curves_give_mirrored_angles() {
        let mut a = joint(DVec2::ZERO, 0.0);
        let mut b = joint(DVec2::ZERO, 0.0);
        let right_curve = drive_circle(&mut a, 20.0, 5.0, 30.0);
        let left_curve = drive_circle(&mut b, -20.0, 5.0, 30.0);
        let (ra, la) = (
            right_curve.last().unwrap().alpha,
            left_curve.last().unwrap().alpha,
        );
        assert!((ra + la).abs() < 0.02, "{ra} vs {la}");
    }

    #[test]
    fn the_joint_limit_holds() {
        let mut t = joint(DVec2::ZERO, 0.0);
        // Spinning the leader on the spot: the segment cannot follow beyond the limit.
        let mut worst: f64 = 0.0;
        for k in 0..200 {
            let heading = f64::from(k) * 0.03;
            let s = t.step(DVec2::new(0.0, f64::from(k) * 0.02), heading, 0.02);
            worst = worst.max(s.alpha.abs());
        }
        assert!(worst <= 52.5f64.to_radians() + 1e-9, "α reached {worst}");
    }

    #[test]
    fn a_standing_vehicle_does_not_drift() {
        let mut t = joint(DVec2::ZERO, 0.3);
        let a = t.step(DVec2::ZERO, 0.3, 0.02);
        let b = (0..500)
            .map(|_| t.step(DVec2::ZERO, 0.3, 0.02))
            .last()
            .unwrap();
        assert!((a.heading - b.heading).abs() < 1e-12 && (a.origin - b.origin).length() < 1e-12);
    }
}
