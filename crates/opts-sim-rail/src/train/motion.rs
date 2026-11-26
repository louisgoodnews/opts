//! Moving the head of a train: forward along the network, and backwards over its trail.

use super::{Moved, Train, MAX_TRAIL};
use opts_sim_pedestrian::{End, PedestrianNet};

impl Train {
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
}
