//! A train on a lane network: head position, route choice at points and the pose of any point a
//! given distance behind the head (bogies, unit origins).

use glam::Vec3;

use crate::pedestrian::{End, LaneId, PedestrianNet};

/// Which way to take at the next point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Branch {
    /// The continuation that bends least (the through route).
    #[default]
    Straight,
    /// The continuation that turns most to the left.
    Left,
    /// The continuation that turns most to the right.
    Right,
}

/// Result of [`Train::advance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moved {
    /// The head moved the whole distance.
    Full,
    /// The track ends (or the next lane may not be entered): the head stopped at the end.
    EndOfTrack,
}

/// Lanes the head has left, newest last (`lane`, walking direction on it).
type Trail = Vec<(LaneId, i8)>;

/// Longest trail kept (lanes); older ones are further back than any consist is long.
const MAX_TRAIL: usize = 64;

/// The head of a train on the network.
#[derive(Debug, Clone, PartialEq)]
pub struct Train {
    /// Current lane of the head.
    pub lane: LaneId,
    /// Arc length of the head on its lane (m).
    pub s: f32,
    /// `+1` towards the lane's end, `-1` towards its start.
    pub dir: i8,
    /// Route choice at the next point.
    pub branch: Branch,
    trail: Trail,
    /// Distance the head has covered on its current lane since it entered it (m).
    on_lane: f32,
}

fn heading(net: &PedestrianNet, lane: LaneId, s: f32, dir: i8) -> f32 {
    let h = net.lane(lane).map_or(0.0, |l| l.at(s).1);
    if dir > 0 {
        h
    } else {
        h + std::f32::consts::PI
    }
}

fn wrap(a: f32) -> f32 {
    let mut a = a % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    }
    if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

impl Train {
    /// Puts the head on `lane` at `s`, walking in `dir`.
    #[must_use]
    pub fn new(lane: LaneId, s: f32, dir: i8) -> Self {
        Self {
            lane,
            s,
            dir: if dir >= 0 { 1 } else { -1 },
            branch: Branch::Straight,
            trail: Vec::new(),
            on_lane: 0.0,
        }
    }

    /// Reverses the direction of travel (the former rear becomes the front is the caller's job:
    /// the trail is dropped and poses behind the head extrapolate).
    pub fn reverse(&mut self) {
        self.dir = -self.dir;
        self.trail.clear();
        self.on_lane = 0.0;
    }

    /// Position and heading (rad, clockwise from north, in the direction of travel) of the head.
    #[must_use]
    pub fn pose(&self, net: &PedestrianNet) -> Option<(Vec3, f32)> {
        let l = net.lane(self.lane)?;
        Some((l.at(self.s).0, heading(net, self.lane, self.s, self.dir)))
    }

    /// Pose of the point `d` metres behind the head along the track the head has run over; before
    /// the beginning of the known trail it extrapolates along the first lane's tangent.
    #[must_use]
    pub fn pose_behind(&self, net: &PedestrianNet, d: f32) -> Option<(Vec3, f32)> {
        let d = d.max(0.0);
        if d <= self.on_lane || self.trail.is_empty() {
            let l = net.lane(self.lane)?;
            let s = self.s - f32::from(self.dir) * d;
            let (p, _) = l.at(s.clamp(0.0, l.length()));
            let over = if s < 0.0 {
                -s
            } else if s > l.length() {
                s - l.length()
            } else {
                0.0
            };
            let h = heading(net, self.lane, s.clamp(0.0, l.length()), self.dir);
            // Beyond the lane's ends (no trail): continue on the tangent.
            let back = Vec3::new(-h.sin(), -h.cos(), 0.0);
            return Some((p + back * over, h));
        }
        let mut remaining = d - self.on_lane;
        for &(lane, dir) in self.trail.iter().rev() {
            let l = net.lane(lane)?;
            let len = l.length();
            if remaining <= len {
                // The lane was left at its far end in direction `dir`.
                let s = if dir > 0 { len - remaining } else { remaining };
                return Some((l.at(s).0, heading(net, lane, s, dir)));
            }
            remaining -= len;
        }
        // Further back than the trail: extrapolate from the oldest lane's entry.
        let &(lane, dir) = self.trail.first()?;
        let l = net.lane(lane)?;
        let s = if dir > 0 { 0.0 } else { l.length() };
        let h = heading(net, lane, s, dir);
        let (p, _) = l.at(s);
        Some((p + Vec3::new(-h.sin(), -h.cos(), 0.0) * remaining, h))
    }

    /// Moves the head `distance` metres forward (never backwards: call [`Train::reverse`] first).
    pub fn advance(&mut self, distance: f32, net: &PedestrianNet) -> Moved {
        let mut left = distance.max(0.0);
        for _ in 0..16 {
            let Some(lane) = net.lane(self.lane) else {
                return Moved::EndOfTrack;
            };
            let len = lane.length();
            let room = if self.dir > 0 { len - self.s } else { self.s };
            if left <= room {
                self.s += f32::from(self.dir) * left;
                self.on_lane += left;
                return Moved::Full;
            }
            left -= room.max(0.0);
            self.on_lane += room.max(0.0);
            let end = if self.dir > 0 { End::End } else { End::Start };
            self.s = if self.dir > 0 { len } else { 0.0 };
            let options = net.continuations(self.lane, end);
            let Some(&(next, entry)) = self.pick(net, &options, end) else {
                return Moved::EndOfTrack;
            };
            self.trail.push((self.lane, self.dir));
            if self.trail.len() > MAX_TRAIL {
                self.trail.remove(0);
            }
            self.lane = next;
            self.on_lane = 0.0;
            match entry {
                End::Start => {
                    self.dir = 1;
                    self.s = 0.0;
                }
                End::End => {
                    self.dir = -1;
                    self.s = net.lane(next).map_or(0.0, |l| l.length());
                }
            }
        }
        Moved::Full
    }

    fn pick<'a>(
        &self,
        net: &PedestrianNet,
        options: &'a [(LaneId, End)],
        _end: End,
    ) -> Option<&'a (LaneId, End)> {
        if options.len() <= 1 {
            return options.first();
        }
        let here = heading(net, self.lane, self.s, self.dir);
        let turn = |o: &(LaneId, End)| {
            let (lane, entry) = *o;
            let s = if entry == End::Start {
                0.0
            } else {
                net.lane(lane).map_or(0.0, |l| l.length())
            };
            let dir = if entry == End::Start { 1 } else { -1 };
            wrap(heading(net, lane, s, dir) - here) // clockwise positive = right
        };
        match self.branch {
            Branch::Straight => options
                .iter()
                .min_by(|a, b| turn(a).abs().total_cmp(&turn(b).abs())),
            Branch::Left => options.iter().min_by(|a, b| turn(a).total_cmp(&turn(b))),
            Branch::Right => options.iter().max_by(|a, b| turn(a).total_cmp(&turn(b))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pedestrian::{FootLane, Travel};

    fn lane(a: Vec3, b: Vec3) -> FootLane {
        FootLane::new(vec![a, b], Travel::Both, 1.0).unwrap()
    }

    /// 100 m north, then a point: straight on (north) or a branch to the north-east.
    fn net() -> PedestrianNet {
        let mut n = PedestrianNet::default();
        n.insert_tile(
            (0, 0),
            vec![
                lane(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)),
                lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.0, 200.0, 0.0)),
                lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(50.0, 150.0, 0.0)),
            ],
        );
        n
    }

    #[test]
    fn straight_branch_takes_the_through_route_and_the_trail_gives_the_rear() {
        let n = net();
        let mut t = Train::new(0, 90.0, 1);
        assert_eq!(t.advance(30.0, &n), Moved::Full);
        assert_eq!(t.lane, 1);
        let (p, h) = t.pose(&n).unwrap();
        assert!((p.y - 120.0).abs() < 1e-3 && h.abs() < 1e-4);
        // 25 m behind the head: 20 m on lane 1 … 5 m back on lane 0.
        let (q, _) = t.pose_behind(&n, 25.0).unwrap();
        assert!((q.y - 95.0).abs() < 1e-3, "{q:?}");
    }

    #[test]
    fn right_branch_takes_the_turn_and_poses_stay_continuous() {
        let n = net();
        let mut t = Train::new(0, 90.0, 1);
        t.branch = Branch::Right;
        t.advance(20.0, &n);
        assert_eq!(t.lane, 2);
        let (_, h) = t.pose(&n).unwrap();
        assert!((h - std::f32::consts::FRAC_PI_4).abs() < 0.05);
        let (q, _) = t.pose_behind(&n, 15.0).unwrap();
        assert!((q - Vec3::new(0.0, 95.0, 0.0)).length() < 0.5, "{q:?}");
    }

    #[test]
    fn the_track_end_stops_the_head() {
        let n = net();
        let mut t = Train::new(1, 90.0, 1);
        assert_eq!(t.advance(50.0, &n), Moved::EndOfTrack);
        assert!((t.s - 100.0).abs() < 1e-3);
    }

    #[test]
    fn rear_before_the_known_trail_extrapolates_along_the_tangent() {
        let n = net();
        let t = Train::new(0, 10.0, 1);
        let (q, _) = t.pose_behind(&n, 30.0).unwrap();
        assert!((q.y + 20.0).abs() < 1e-3, "{q:?}");
    }
}
