//! The ground methods of the simulation: probes and applying the ground.

use glam::{DVec2, DVec3};

use super::hitch_pose::{hitch_height, hitch_plan_shift, trailer_body, AxleGround};
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
    /// consist unit `k` (`None`: no ground there: the unit keeps its height but levels out, pitch
    /// and roll fade to zero, because a tilt left from the last ground would go on pulling the
    /// vehicle down a slope that is no longer there). A unit without a state yet starts on its goal.
    /// A following unit's goal is the rigid pose of [`VehicleSimulation::coupled_goal`] (its coupling
    /// on the hitch of the unit ahead, its axles on the ground), or its own ground goal when the
    /// unit ahead has none. The filters run with `dt`, the fixed step, so the result depends on
    /// nothing else.
    pub fn apply_ground(&mut self, samples: &[Option<GroundSample>], dt: f32) {
        if let Some(s) = samples.first() {
            settle(&mut self.head_body, s.as_ref().map(GroundSample::goal), dt);
        }
        let (mut lead_pos, mut lead_heading) = self.chassis_pose();
        let mut lead = samples
            .first()
            .and_then(Option::as_ref)
            .map(GroundSample::goal)
            .or(self.head_body);
        for k in 0..self.consist_bodies.len() {
            let sample = samples.get(k + 1).and_then(Option::as_ref);
            let goal = match (lead, sample) {
                (Some(l), Some(s)) => Some(self.couple(k, (lead_pos, lead_heading), &l, s)),
                (_, s) => s.map(GroundSample::goal),
            };
            settle(&mut self.consist_bodies[k], goal, dt);
            lead = goal.or(self.consist_bodies[k]);
            lead_pos = self.consist[k].origin;
            lead_heading = self.consist[k].heading;
        }
    }

    /// Couples consist unit `k` behind the leader at `leader` (`(pos, heading)`) in the pose `lead`
    /// and stands it on `sample`: its hitch moves by the leader's tilt, its origin is placed behind
    /// that hitch, and the rigid goal of [`super::hitch_pose::trailer_body`] is returned.
    fn couple(
        &mut self,
        k: usize,
        leader: (DVec2, f64),
        lead: &UnitBody,
        sample: &GroundSample,
    ) -> UnitBody {
        let unit = &self.consist[k];
        let [hitch_z, front_z] = self.consist_couplings.get(k).copied().unwrap_or([0.0, 0.0]);
        let hitch = DVec3::new(unit.hitch_local.x, unit.hitch_local.y, hitch_z);
        let front = DVec3::new(unit.front_local.x, unit.front_local.y, front_z);
        let [left, right, _, _] = sample.heights.map(f64::from);
        let axle = AxleGround {
            rear_y: f64::from(sample.probe.rear_y),
            track: f64::from(sample.probe.track_m),
            left,
            right,
        };
        let shift = hitch_plan_shift(leader.1, lead, hitch);
        let unit = &mut self.consist[k];
        unit.hitch_shift = shift;
        unit.place(leader.0, leader.1);
        trailer_body(hitch_height(lead, hitch), front, axle)
    }
}

/// Settles `body` towards `goal`: a state without a goal levels out, a missing state starts on it.
fn settle(body: &mut Option<UnitBody>, goal: Option<UnitBody>, dt: f32) {
    match (body.as_mut(), goal) {
        (Some(b), Some(g)) => b.settle(g, dt),
        (None, Some(g)) => *body = Some(g),
        (Some(b), None) => b.settle(
            UnitBody {
                elevation: b.elevation,
                pitch: 0.0,
                roll: 0.0,
            },
            dt,
        ),
        (None, None) => {}
    }
}
