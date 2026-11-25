//! walkers: their advance along the lanes and their pose.

use super::*;

/// Result of [`Walker::advance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    /// The walker is on a lane.
    Moving,
    /// The lane ended in a one-way dead end or disappeared (its tile was unloaded).
    Finished,
}

/// A pedestrian on the network.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Walker {
    /// Current lane.
    pub lane: LaneId,
    /// Arc length on the lane (m).
    pub s: f32,
    /// `+1` walking towards the end, `-1` towards the start.
    pub dir: i8,
    /// Walking speed (m/s).
    pub speed: f32,
    /// Gait phase (rad); advanced by the caller from the speed and the stride.
    pub phase: f32,
    /// How far from the lane's centre line the walker keeps, to the right of the walking
    /// direction, as a fraction of half the lane width (see [`Walker::pose`]). `0` walks on the
    /// line.
    pub lateral: f32,
}

/// The range a new walker's [`Walker::lateral`] is drawn from: not on the centre line, not at the
/// very edge, so walkers in opposite directions pass at least `0.3` of a half-width apart.
const LATERAL_RANGE: (f32, f32) = (0.15, 0.85);

/// Walkers keep at most this far (m) from the centre line, however wide the lane is (a wide
/// lane is a plaza, not a pavement).
pub(super) const MAX_OFFSET_M: f32 = 1.5;

impl Walker {
    /// Places a walker on `lane` at `s`, choosing an allowed direction.
    #[must_use]
    pub fn new(
        net: &PedestrianNet,
        lane: LaneId,
        s: f32,
        speed: f32,
        rng: &mut u32,
    ) -> Option<Self> {
        let dir = match net.lane(lane)?.travel {
            Travel::Forward => 1,
            Travel::Reverse => -1,
            Travel::Both => {
                if next_f32(rng) < 0.5 {
                    1
                } else {
                    -1
                }
            }
        };
        Some(Self {
            lane,
            s,
            dir,
            speed,
            phase: next_f32(rng) * std::f32::consts::TAU,
            lateral: LATERAL_RANGE.0 + next_f32(rng) * (LATERAL_RANGE.1 - LATERAL_RANGE.0),
        })
    }

    /// Moves the walker `dt` seconds along the network.
    pub fn advance(&mut self, dt: f32, net: &PedestrianNet, rng: &mut u32) -> Advance {
        let mut left = self.speed * dt;
        // A long step may cross several short lanes; bounded to stay finite.
        for _ in 0..8 {
            let Some(lane) = net.lane(self.lane) else {
                return Advance::Finished;
            };
            let len = lane.length();
            let room = if self.dir > 0 { len - self.s } else { self.s };
            if left <= room {
                self.s += f32::from(self.dir) * left;
                return Advance::Moving;
            }
            left -= room.max(0.0);
            let end = if self.dir > 0 { End::End } else { End::Start };
            self.s = if self.dir > 0 { len } else { 0.0 };
            let next = net.continuations(self.lane, end);
            if next.is_empty() {
                let back_ok = match (lane.travel, self.dir) {
                    (Travel::Both, _) => true,
                    (Travel::Forward, d) if d < 0 => true,
                    (Travel::Reverse, d) if d > 0 => true,
                    _ => false,
                };
                // Turning round is only allowed on a lane that may be walked in both ways.
                if lane.travel == Travel::Both && back_ok {
                    self.dir = -self.dir;
                    continue;
                }
                return Advance::Finished;
            }
            let (id, entry) = next[(next_f32(rng) * next.len() as f32) as usize % next.len()];
            self.lane = id;
            match entry {
                End::Start => {
                    self.dir = 1;
                    self.s = 0.0;
                }
                End::End => {
                    self.dir = -1;
                    self.s = net.lane(id).map_or(0.0, FootLane::length);
                }
            }
        }
        Advance::Moving
    }

    /// World position and heading (radians, clockwise from north, in the walking direction).
    ///
    /// The position is beside the lane's centre line, on the right of the walking direction by
    /// [`Walker::lateral`] times half the lane width (at most 1.5 m), so walkers in
    /// opposite directions pass on different sides. Walkers do not react to each other: two
    /// walking the same way at the same place and offset still overlap.
    #[must_use]
    pub fn pose(&self, net: &PedestrianNet) -> Option<(Vec3, f32)> {
        let lane = net.lane(self.lane)?;
        let (p, h) = lane.at(self.s);
        let heading = if self.dir > 0 {
            h
        } else {
            h + std::f32::consts::PI
        };
        let half = if lane.width.is_finite() {
            (lane.width * 0.5).clamp(0.0, MAX_OFFSET_M)
        } else {
            0.0
        };
        // Heading is clockwise from north with x east and y north: the right-hand side is
        // `(cos, -sin)`.
        let right = Vec3::new(heading.cos(), -heading.sin(), 0.0);
        Some((p + right * (self.lateral * half), heading))
    }
}
