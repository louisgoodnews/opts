//! Switch detection and branch selection at rail track junctions.

use super::*;

impl Train {
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

    /// The lane to back onto among `options`: the one the vehicle would still face the same
    /// way on (a backing vehicle keeps its heading), `branch` deciding between several.
    /// The options that are not closed by rail points; all of them when every one is (a train is
    /// never stranded) or none is.
    pub(super) fn passable<'a>(
        net: &PedestrianNet,
        options: &'a [(LaneId, End)],
    ) -> Vec<&'a (LaneId, End)> {
        let open: Vec<_> = options.iter().filter(|(l, _)| !net.is_closed(*l)).collect();
        if open.is_empty() {
            options.iter().collect()
        } else {
            open
        }
    }

    pub(super) fn pick_back<'a>(
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

    pub(super) fn pick<'a>(
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
    pub(super) fn prefer<'a>(
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
