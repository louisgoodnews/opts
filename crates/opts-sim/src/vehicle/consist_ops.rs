//! The consist methods of the simulation: trailers, masses, couplings.

use super::*;

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

    /// Removes all following units.
    pub fn clear_consist(&mut self) {
        self.consist.clear();
        self.trailer_loads.clear();
        self.consist_bodies.clear();
        self.head_body = None;
    }

    /// Mass of the whole consist the longitudinal dynamics act on (kg): the chassis' own mass
    /// plus the following units' (design D2, `Σ mᵢ`) plus the people on board.
    #[must_use]
    pub fn consist_mass_kg(&self) -> f32 {
        self.chassis.total_mass_kg
            + self.trailer_loads.iter().map(|u| u.mass_kg).sum::<f32>()
            + self.passenger_mass_kg
    }

    /// Mass of the head unit alone (kg): the chassis' own mass.
    #[must_use]
    pub fn head_mass_kg(&self) -> f32 {
        self.chassis.total_mass_kg
    }

    /// Mass standing on driven axles over all units (kg), `friction_loads_n().0 / g`.
    #[must_use]
    pub fn driven_mass_kg(&self) -> f32 {
        self.friction_loads_n().0 / GRAVITY
    }

    /// Declares the loads of the following units *without* poses (headless dynamics, tests):
    /// replaces `trailer_loads`. A consist with poses gets its loads through
    /// [`VehicleSimulation::attach_trailer`], which keeps `consist` and `trailer_loads` in step.
    pub fn set_trailers(&mut self, units: Vec<UnitLoad>) {
        self.trailer_loads = units;
    }

    /// Summed static normal force `(driven axles, braked axles)` over all units (N): what the
    /// tyres can transmit is `ROAD_FRICTION` times these. The chassis' axles carry its own mass
    /// (distributed by the suspension model), each following unit its own; every axle brakes.
    #[must_use]
    pub fn friction_loads_n(&self) -> (f32, f32) {
        let positions: Vec<f32> = self.axles.iter().map(|a| a.config.longitudinal_m).collect();
        let head = self
            .suspension
            .compute_static_loads(&positions, self.chassis.total_mass_kg);
        // A head without any axle marked driven counts as driven on all of them (the file does
        // not say; do not take away all traction).
        let head_has_driven = self.axles.iter().any(|a| a.config.is_driven);
        let mut axles: Vec<(UnitAxle, f32)> = self
            .axles
            .iter()
            .zip(head)
            .map(|(a, n)| {
                (
                    UnitAxle {
                        longitudinal_m: a.config.longitudinal_m,
                        is_driven: a.config.is_driven || !head_has_driven,
                        rated_n: a.config.max_force_n,
                    },
                    n,
                )
            })
            .collect();
        for unit in &self.trailer_loads {
            axles.extend(unit.axles.iter().copied().zip(unit.axle_loads_n()));
        }
        driven_and_braked_load_n(&axles)
    }

    /// Force the joint of consist unit `k` carries (N, report only; nothing is fed back): the
    /// mass behind it accelerated with the chassis plus its rolling resistance,
    /// `F = Σ_{j≥k} mⱼ (a + sign(v)·Crr·g)`. `None` for a missing unit.
    #[must_use]
    pub fn hitch_force_n(&self, k: usize) -> Option<f32> {
        let behind: f32 = self
            .trailer_loads
            .get(k..)
            .filter(|rest| !rest.is_empty())?
            .iter()
            .map(|u| u.mass_kg)
            .sum();
        let v = self.chassis.forward_speed_m_s();
        let direction = if v.abs() > 0.01 { v.signum() } else { 0.0 };
        let a = self.chassis.local_acceleration.y;
        Some(behind * (a + direction * drivetrain::ROLLING_RESISTANCE_COEFF * GRAVITY))
    }

    /// Yaw angle of consist unit `k` against the unit ahead in radians (positive = turned
    /// clockwise), the value published as `articulation_<k>_alpha`; `None` for a missing unit.
    #[must_use]
    pub fn consist_alpha_rad(&self, k: usize) -> Option<f64> {
        let unit = self.consist.get(k)?;
        Some(unit.alpha_against(self.leader_pose_of(k).1))
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
