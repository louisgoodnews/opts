//! The consist methods of the simulation: trailers, masses, couplings.

use super::*;

mod forces;
mod mass;

impl VehicleSimulation {
    /// Pose `(origin, heading)` of the unit ahead of consist unit `k`: the chassis for `k = 0`.
    fn leader_pose_of(&self, k: usize) -> (DVec2, f64) {
        match k.checked_sub(1).and_then(|j| self.consist.get(j)) {
            Some(ahead) => (ahead.origin, ahead.heading),
            None => self.chassis_pose(),
        }
    }

    /// Adds a following unit behind the last one (or behind the chassis), hanging straight.
    ///
    /// `hitch_local` is the rear coupling of the unit ahead and `front_local` the front coupling
    /// of the new unit, both `(x, y)` in their own frames; `axle_distance` is the distance from
    /// the hitch to the new unit's rearmost axle and `max_alpha_deg` the joint limit. `load` is
    /// the unit's mass and axles (`None`: the unit does not take part in the dynamics).
    pub fn attach_trailer(
        &mut self,
        hitch_local: DVec2,
        front_local: DVec2,
        axle_distance: f64,
        max_alpha_deg: f64,
        load: Option<UnitLoad>,
    ) {
        let (pos, heading) = self.leader_pose_of(self.consist.len());
        self.consist.push(TrailerKinematics::new(
            hitch_local,
            front_local,
            axle_distance,
            max_alpha_deg,
            pos,
            heading,
        ));
        self.consist_bodies.push(None);
        self.consist_couplings.push([0.0, 0.0]);
        self.trailer_loads.push(load.unwrap_or(UnitLoad {
            mass_kg: 0.0,
            axles: Vec::new(),
        }));
    }

    /// The fixed-step body of consist unit `k` (`None` before the first ground sample).
    #[must_use]
    pub fn consist_body(&self, k: usize) -> Option<UnitBody> {
        self.consist_bodies.get(k).copied().flatten()
    }

    /// `(β, γ)` in degrees of joint `k` (unit `k` against the unit ahead), the values behind
    /// `articulation_<k>_beta` and `_gamma` before the joint's `β` limits; `None` until both
    /// units have a body.
    #[must_use]
    pub fn joint_angles_deg(&self, k: usize) -> Option<(f32, f32)> {
        let leader = match k.checked_sub(1) {
            Some(j) => self.consist_body(j)?,
            None => self.head_body?,
        };
        let unit = self.consist_body(k)?;
        Some((
            joint_beta_deg(leader.pitch, unit.pitch),
            (unit.roll - leader.roll).to_degrees(),
        ))
    }

    /// Heights of the couplings of consist unit `k` above its origin (m, OMSI): `(hitch_z, front_z)`,
    /// the rear coupling of the unit ahead in its frame and the unit's own front coupling. Zero
    /// until set (the couplings of a bus file are set by the host, see [`super::hitch_pose`]).
    pub fn set_consist_couplings_z(&mut self, k: usize, hitch_z: f64, front_z: f64) {
        if let Some(c) = self.consist_couplings.get_mut(k) {
            *c = [hitch_z, front_z];
        }
    }

    /// Removes all following units.
    pub fn clear_consist(&mut self) {
        self.consist.clear();
        self.trailer_loads.clear();
        self.consist_bodies.clear();
        self.consist_couplings.clear();
        self.head_body = None;
    }

    /// Declares the loads of the following units *without* poses (headless dynamics, tests):
    /// replaces `trailer_loads`. A consist with poses gets its loads through
    /// [`VehicleSimulation::attach_trailer`], which keeps `consist` and `trailer_loads` in step.
    pub fn set_trailers(&mut self, units: Vec<UnitLoad>) {
        self.trailer_loads = units;
    }

    /// Published yaw angle of consist unit `k` against the unit ahead, in radians, the value
    /// written as `articulation_<k>_alpha`. Positive means turned counter-clockwise, a right-hand
    /// curve (OMSI's articulation convention). The simulation keeps the clockwise angle
    /// internally ([`TrailerKinematics::alpha_against`]), so the sign is flipped here only.
    /// `None` for a missing unit.
    #[must_use]
    pub fn consist_alpha_rad(&self, k: usize) -> Option<f64> {
        let unit = self.consist.get(k)?;
        Some(-unit.alpha_against(self.leader_pose_of(k).1))
    }

    /// Re-seeds every following unit straight behind its leader, without velocity.
    pub fn reseed_consist(&mut self) {
        for k in 0..self.consist.len() {
            let (pos, heading) = self.leader_pose_of(k);
            self.consist[k].reseed(pos, heading);
        }
        // The next ground sample seeds the bodies on the new ground.
        self.head_body = None;
        self.consist_bodies.iter_mut().for_each(|b| *b = None);
    }
}
