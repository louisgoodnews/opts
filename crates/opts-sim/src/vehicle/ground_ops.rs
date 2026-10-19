//! The ground methods of the simulation: probes and applying the ground.

use super::*;

impl VehicleSimulation {
    /// The points to sample the ground at, one probe per unit: the chassis first, then each
    /// following unit (see [`ground`]).
    #[must_use]
    pub fn ground_probes(&self) -> Vec<GroundProbe> {
        let ends = self
            .axles
            .iter()
            .map(|a| a.config.longitudinal_m)
            .fold((f32::MIN, f32::MAX), |(hi, lo), y| (hi.max(y), lo.min(y)));
        let track = self
            .axles
            .iter()
            .map(|a| a.config.track_width_m)
            .fold(0.0_f32, f32::max);
        let track_m = if track > 0.5 { track } else { 2.0 };
        let (front_y, rear_y) = if ends.0 > f32::MIN {
            (ends.0, ends.1)
        } else {
            (3.0, -3.0)
        };
        let mut probes = vec![GroundProbe {
            origin: self.chassis.position,
            heading: self.chassis.heading_rad,
            rear_y,
            front_y,
            track_m,
        }];
        probes.extend(self.consist.iter().map(|u| GroundProbe {
            origin: Vec2::new(u.origin.x as f32, u.origin.y as f32),
            heading: u.heading as f32,
            rear_y: u.front_local.y as f32 - u.axle_distance as f32,
            front_y: u.front_local.y as f32,
            track_m,
        }));
        probes
    }

    /// Takes one fixed step of ground following: `samples[0]` is the chassis, `samples[k + 1]`
    /// consist unit `k` (`None`: no ground there, the unit keeps its state). A unit without a
    /// state yet starts on its goal. The filters run with `dt`, the fixed step, so the result
    /// depends on nothing else.
    pub fn apply_ground(&mut self, samples: &[Option<GroundSample>], dt: f32) {
        let settle = |body: &mut Option<UnitBody>, sample: &Option<GroundSample>| {
            let Some(sample) = sample else { return };
            let goal = sample.goal();
            match body {
                Some(b) => b.settle(goal, dt),
                None => *body = Some(goal),
            }
        };
        if let Some(s) = samples.first() {
            settle(&mut self.head_body, s);
        }
        for (k, body) in self.consist_bodies.iter_mut().enumerate() {
            if let Some(s) = samples.get(k + 1) {
                settle(body, s);
            }
        }
    }
}
