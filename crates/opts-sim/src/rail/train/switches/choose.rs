//! Choosing the way at points: the lane to back onto, the lane to take and the preferred option.

use super::*;

impl Train {
    /// The lane to back onto among `options`: the one the vehicle would still face the same
    /// way on (a backing vehicle keeps its heading), `branch` deciding between several.
    /// The options that are not closed by rail points; all of them when every one is (a train is
    /// never stranded) or none is.
    pub(in super::super) fn passable<'a>(
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

    pub(in super::super) fn pick_back<'a>(
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

    pub(in super::super) fn pick<'a>(
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
    pub(in super::super) fn prefer<'a>(
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
