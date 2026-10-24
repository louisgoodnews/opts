//! A train on a lane network: head position, route choice at points and the pose of any point a
//! given distance behind the head (bogies, unit origins).

use std::collections::VecDeque;

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

/// Points ahead of a train.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchAhead {
    /// Metres from the head to the points.
    pub distance_m: f32,
    /// The ways open there, straight first.
    pub branches: Vec<Branch>,
    /// The lane the train would take there if the points were set for it (its route or chosen
    /// branch), whatever the position of the points.
    pub wanted: LaneId,
}

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
    /// The lanes the head takes next, one after the other. At a point the head enters the next
    /// lane of this route when it is among the ways open ([`PedestrianNet::shortest_path`] gives
    /// one); `branch` decides where the route does not.
    pub route: VecDeque<LaneId>,
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
            route: VecDeque::new(),
            trail: Vec::new(),
            on_lane: 0.0,
        }
    }

    /// The lanes the train occupies: the head's and the ones it has left (the rear may still be
    /// on them).
    pub fn lanes_occupied(&self) -> impl Iterator<Item = LaneId> + '_ {
        std::iter::once(self.lane).chain(self.trail.iter().map(|(l, _)| *l))
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
            if self.route.front() == Some(&next) {
                self.route.pop_front();
            }
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

    /// The next points ahead within `within` metres: how far they are and which ways lead on
    /// from them (the way that continues most straight is [`Branch::Straight`], the others lie to
    /// its left or right). `None` when the track runs on without a choice.
    #[must_use]
    pub fn next_switch(&self, net: &PedestrianNet, within: f32) -> Option<SwitchAhead> {
        let mut lane = self.lane;
        let mut dir = self.dir;
        let mut s = self.s;
        let mut travelled = 0.0f32;
        for _ in 0..12 {
            let l = net.lane(lane)?;
            let len = l.length();
            travelled += if dir > 0 { len - s } else { s };
            if travelled > within {
                return None;
            }
            let end = if dir > 0 { End::End } else { End::Start };
            let options = net.continuations(lane, end);
            let here = heading(net, lane, if dir > 0 { len } else { 0.0 }, dir);
            let turn = |o: &(LaneId, End)| {
                let (id, entry) = *o;
                let (at, d) = if entry == End::Start {
                    (0.0, 1)
                } else {
                    (net.lane(id).map_or(0.0, |l| l.length()), -1)
                };
                wrap(heading(net, id, at, d) - here)
            };
            if options.len() > 1 {
                let straight = options
                    .iter()
                    .min_by(|a, b| turn(a).abs().total_cmp(&turn(b).abs()))?;
                let mut branches = vec![Branch::Straight];
                for o in options.iter().filter(|o| *o != straight) {
                    let b = if turn(o) < turn(straight) {
                        Branch::Left
                    } else {
                        Branch::Right
                    };
                    if !branches.contains(&b) {
                        branches.push(b);
                    }
                }
                let wanted = self
                    .prefer(net, options.iter().collect(), here)
                    .map_or(straight.0, |o| o.0);
                return Some(SwitchAhead {
                    distance_m: travelled,
                    branches,
                    wanted,
                });
            }
            let &(next, entry) = options.first()?;
            lane = next;
            if entry == End::Start {
                dir = 1;
                s = 0.0;
            } else {
                dir = -1;
                s = net.lane(next).map_or(0.0, |l| l.length());
            }
        }
        None
    }

    /// Moves the head `distance` metres backwards along the track it ran over (a vehicle backing
    /// up keeps facing the way it faced): the trail is taken back lane by lane, and beyond the
    /// known trail the lane behind the first one is looked up.
    pub fn retreat(&mut self, distance: f32, net: &PedestrianNet) -> Moved {
        let mut left = distance.max(0.0);
        for _ in 0..16 {
            let Some(lane) = net.lane(self.lane) else {
                return Moved::EndOfTrack;
            };
            let len = lane.length();
            let rear = if self.dir > 0 { self.s } else { len - self.s };
            let room = if self.trail.is_empty() {
                rear
            } else {
                self.on_lane.min(rear)
            };
            if left <= room {
                self.s -= f32::from(self.dir) * left;
                self.on_lane = (self.on_lane - left).max(0.0);
                return Moved::Full;
            }
            left -= room.max(0.0);
            if let Some((back, back_dir)) = self.trail.pop() {
                // The lane before was left at its far end in `back_dir`: back onto it.
                let Some(l) = net.lane(back) else {
                    return Moved::EndOfTrack;
                };
                self.lane = back;
                self.dir = back_dir;
                self.s = if back_dir > 0 { l.length() } else { 0.0 };
                self.on_lane = l.length();
                continue;
            }
            let end = if self.dir > 0 { End::Start } else { End::End };
            self.s = if self.dir > 0 { 0.0 } else { len };
            let options = net.continuations(self.lane, end);
            let Some(&(next, entry)) = self.pick_back(net, &options) else {
                return Moved::EndOfTrack;
            };
            let next_len = net.lane(next).map_or(0.0, |l| l.length());
            self.lane = next;
            self.on_lane = 0.0;
            if entry == End::End {
                self.dir = 1;
                self.s = next_len;
            } else {
                self.dir = -1;
                self.s = 0.0;
            }
        }
        Moved::Full
    }

    /// The lane to back onto among `options`: the one the vehicle would still face the same
    /// way on (a backing vehicle keeps its heading), `branch` deciding between several.
    /// The options that are not closed by rail points; all of them when every one is (a train is
    /// never stranded) or none is.
    fn passable<'a>(net: &PedestrianNet, options: &'a [(LaneId, End)]) -> Vec<&'a (LaneId, End)> {
        let open: Vec<_> = options.iter().filter(|(l, _)| !net.is_closed(*l)).collect();
        if open.is_empty() {
            options.iter().collect()
        } else {
            open
        }
    }

    fn pick_back<'a>(
        &self,
        net: &PedestrianNet,
        options: &'a [(LaneId, End)],
    ) -> Option<&'a (LaneId, End)> {
        let options = Self::passable(net, options);
        if options.len() <= 1 {
            return options.first().copied();
        }
        let here = heading(net, self.lane, self.s, self.dir);
        let turn = |o: &&(LaneId, End)| {
            let (lane, entry) = **o;
            let h = if entry == End::End {
                heading(net, lane, net.lane(lane).map_or(0.0, |l| l.length()), 1)
            } else {
                heading(net, lane, 0.0, -1)
            };
            wrap(h - here)
        };
        match self.branch {
            Branch::Straight => options
                .into_iter()
                .min_by(|a, b| turn(a).abs().total_cmp(&turn(b).abs())),
            Branch::Left => options
                .into_iter()
                .min_by(|a, b| turn(a).total_cmp(&turn(b))),
            Branch::Right => options
                .into_iter()
                .max_by(|a, b| turn(a).total_cmp(&turn(b))),
        }
    }

    fn pick<'a>(
        &self,
        net: &PedestrianNet,
        options: &'a [(LaneId, End)],
        _end: End,
    ) -> Option<&'a (LaneId, End)> {
        let options = Self::passable(net, options);
        if options.len() <= 1 {
            return options.first().copied();
        }
        self.prefer(net, options, heading(net, self.lane, self.s, self.dir))
    }

    /// The option the route or the chosen branch asks for, `here` being the heading at the fork.
    fn prefer<'a>(
        &self,
        net: &PedestrianNet,
        options: Vec<&'a (LaneId, End)>,
        here: f32,
    ) -> Option<&'a (LaneId, End)> {
        if let Some(next) = self.route.front() {
            if let Some(o) = options.iter().find(|(l, _)| l == next) {
                return Some(o);
            }
        }
        let turn = |o: &&(LaneId, End)| {
            let (lane, entry) = **o;
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
                .into_iter()
                .min_by(|a, b| turn(a).abs().total_cmp(&turn(b).abs())),
            Branch::Left => options
                .into_iter()
                .min_by(|a, b| turn(a).total_cmp(&turn(b))),
            Branch::Right => options
                .into_iter()
                .max_by(|a, b| turn(a).total_cmp(&turn(b))),
        }
    }
}

#[cfg(test)]
#[path = "train_tests.rs"]
mod tests;
