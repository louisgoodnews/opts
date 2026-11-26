//! Switch detection and branch selection at rail track junctions.

use super::*;

mod choose;

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
}
