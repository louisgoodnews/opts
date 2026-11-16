//! The path point under an agent and its axles: the front axle on the path, the rear axle at
//! the wheelbase, and the steering angle.

use super::*;

impl TrafficWorld {
    /// The point `back` metres behind agent `i`'s front bumper along its path, with the path's
    /// heading there and the sideways offset of a lane change: on the current lane, else on the
    /// previous one, else on the extension of the lane's start tangent.
    fn path_point(&self, i: usize, back: f64) -> Option<(DVec2, f64)> {
        let a = &self.agents[i];
        let g = &self.graph.lane(a.lane)?.geometry;
        let s = a.s - back;
        let (p, h) = if s >= 0.0 {
            (g.point_at(s), g.heading_at(s))
        } else if let Some(prev) = a
            .prev_lane
            .and_then(|l| self.graph.lane(l))
            .filter(|l| l.length() + s >= 0.0)
        {
            let t = prev.length() + s;
            (prev.geometry.point_at(t), prev.geometry.heading_at(t))
        } else {
            let h = g.heading;
            (g.start + DVec2::new(h.sin(), h.cos()) * s, h)
        };
        Some((p + DVec2::new(h.cos(), -h.sin()) * a.lateral, h))
    }

    /// Moves agent `i`'s axles: the front axle onto its path, the rear axle after it at the
    /// wheelbase. `travel` (m) is how far the front axle may have moved since the last update;
    /// a larger jump (a rebuilt graph, a teleport) places the rear axle afresh on the path's
    /// tangent, as on spawn.
    pub(crate) fn update_axles(&mut self, i: usize, travel: f64) {
        let p = self.agents[i].params;
        let Some((on_path, h)) = self.path_point(i, p.front_overhang) else {
            return;
        };
        let Some((target, _)) = self.path_point(i, p.front_overhang - FRONT_PURSUIT_M) else {
            return;
        };
        let a = &mut self.agents[i];
        let l = p.wheelbase.max(0.1);
        let jumped = !a.axles_ready
            || on_path.distance(a.front_axle) > travel + AXLE_RESET_M
            || (a.front_axle.distance(a.rear_axle) - l).abs() > AXLE_RESET_M;
        if jumped {
            a.front_axle = on_path;
            a.rear_axle = on_path - DVec2::new(h.sin(), h.cos()) * l;
        } else {
            // The front axle pursues a point just ahead on the path (on an arc it stays within
            // d²/2R of the lane, a kink at a lane join is rounded off), the rear axle the front
            // one; both in sub-steps of at most `AXLE_SUBSTEP_M` of travel.
            let n = (a.pursuit.distance(target) / AXLE_SUBSTEP_M)
                .ceil()
                .clamp(1.0, 64.0);
            let from = a.pursuit;
            for k in 1..=n as usize {
                let t = from.lerp(target, k as f64 / n);
                let d = t - a.front_axle;
                if d.length_squared() > 1e-12 {
                    a.front_axle = t - d.normalize() * FRONT_PURSUIT_M;
                }
                let d = a.front_axle - a.rear_axle;
                if d.length_squared() > 1e-12 {
                    a.rear_axle = a.front_axle - d.normalize() * l;
                }
            }
        }
        a.pursuit = target;
        a.axles_ready = true;
        let steer = h - a.heading();
        a.steer =
            (steer + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
    }
}
