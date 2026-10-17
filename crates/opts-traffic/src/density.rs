//! Spawn weights and traffic density helpers.
//!
//! The data comes from `ailists.cfg` (weights), `unsched_vehgroups.txt` (groups and their default
//! density), `unsched_trafficdens.txt` (curves) and the `[rule] trafficdensity` records of the
//! tiles (per path and group), parsed in `opts_core::formats::map`. How OMSI turns a density
//! factor into vehicles per kilometre is not documented ❓; hosts choose their own scale
//! (`docs/src/architecture/ai-traffic.md`, density model).

use crate::rng::Rng;

/// A piecewise-linear density curve over the hours of a day (`[trafficdensity]` points).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DensityCurve {
    points: Vec<(f32, f32)>,
}

impl DensityCurve {
    /// Builds a curve; points are sorted by hour.
    #[must_use]
    pub fn new(mut points: Vec<(f32, f32)>) -> Self {
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self { points }
    }

    /// Density at `hour` (clock hours, wraps at 24; `1.0` for an empty curve).
    #[must_use]
    pub fn at(&self, hour: f32) -> f32 {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else {
            return 1.0;
        };
        let h = hour.rem_euclid(24.0);
        if h <= first.0 {
            return first.1;
        }
        self.points
            .windows(2)
            .find(|w| h <= w[1].0)
            .map_or(last.1, |w| {
                let t = if w[1].0 > w[0].0 {
                    (h - w[0].0) / (w[1].0 - w[0].0)
                } else {
                    1.0
                };
                w[0].1 + (w[1].1 - w[0].1) * t
            })
    }
}

/// Most vehicle groups a lane keeps density rules for (the corpus uses flags `0`–`9`).
pub const MAX_DENSITY_GROUPS: usize = 16;

/// Density class "Medium": the density of the first group on a path without a rule when its
/// default is `1` (`unsched_vehgroups.txt`: "if this is the first UVG, then 1 = use density class
/// Medium by default"). Its value is not documented ❓; OPTS uses `1.0`.
pub const MEDIUM_DENSITY: f64 = 1.0;

/// The `[rule] trafficdensity` values of a lane by vehicle group: the rule's flag is the 0-based
/// index into `unsched_vehgroups.txt` (all 297k rules of the installed maps are in range); the
/// last rule for a group wins. Values are kept as written (`3` is three times the normal density).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LaneDensity {
    set: u16,
    values: [f32; MAX_DENSITY_GROUPS],
}

impl LaneDensity {
    /// The rule for `group`, if the lane has one.
    #[must_use]
    pub fn get(&self, group: usize) -> Option<f64> {
        (group < MAX_DENSITY_GROUPS && self.set & (1 << group) != 0)
            .then(|| f64::from(self.values[group]))
    }

    /// Sets the rule for `group`; `false` (and nothing set) for a group beyond
    /// [`MAX_DENSITY_GROUPS`].
    pub fn set(&mut self, group: usize, value: f64) -> bool {
        if group >= MAX_DENSITY_GROUPS {
            return false;
        }
        self.set |= 1 << group;
        self.values[group] = value as f32;
        true
    }

    /// Whether the lane has no density rule at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.set == 0
    }

    /// The rules as `(group, value)` in group order.
    pub fn iter(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        (0..MAX_DENSITY_GROUPS).filter_map(|g| self.get(g).map(|v| (g, v)))
    }

    /// Whether a rule closes the lane to `group` (its value is `0`). Lane changes and route
    /// choices into such a lane are left out; a lane without a rule for the group is open.
    #[must_use]
    pub fn closed_for(&self, group: usize) -> bool {
        self.get(group).is_some_and(|v| v <= 0.0)
    }

    /// Density of `group` on the lane: its rule if there is one, else the group's default from
    /// `unsched_vehgroups.txt` (`defaults[group]`): `0` none, `k ≥ 1` the density of group
    /// `k − 1` on this lane, and for the first group `1` the class "Medium"
    /// ([`MEDIUM_DENSITY`]). A group beyond `defaults` has no traffic; a reference cycle ends in
    /// "Medium".
    #[must_use]
    pub fn for_group(&self, group: usize, defaults: &[u32]) -> f64 {
        let mut g = group;
        for _ in 0..=defaults.len() {
            if let Some(v) = self.get(g) {
                return v.max(0.0);
            }
            match defaults.get(g).copied() {
                None | Some(0) => return 0.0,
                Some(k) => {
                    let next = k as usize - 1;
                    if next == g {
                        return MEDIUM_DENSITY;
                    }
                    g = next;
                }
            }
        }
        MEDIUM_DENSITY
    }
}

/// Number of vehicles to keep per group: `wanted[g]` (fractional) scaled down proportionally when
/// their sum exceeds `cap`. Returns the per-group targets and whether the cap bound.
#[must_use]
pub fn group_targets(wanted: &[f64], cap: usize) -> (Vec<f64>, bool) {
    let total: f64 = wanted.iter().map(|w| w.max(0.0)).sum();
    let cap = cap as f64;
    if total <= cap || total <= 0.0 {
        return (wanted.iter().map(|w| w.max(0.0)).collect(), false);
    }
    let k = cap / total;
    (wanted.iter().map(|w| w.max(0.0) * k).collect(), true)
}

/// Picks an index with probability proportional to its weight. Missing weights are `1`.
/// Returns `None` for an empty list or when every weight is zero.
pub fn weighted_pick(weights: &[Option<f32>], rng: &mut Rng) -> Option<usize> {
    let w = |x: &Option<f32>| f64::from(x.unwrap_or(1.0).max(0.0));
    let total: f64 = weights.iter().map(w).sum();
    if total <= 0.0 {
        return None;
    }
    let mut target = rng.next_f64() * total;
    for (i, x) in weights.iter().enumerate() {
        target -= w(x);
        if target < 0.0 {
            return Some(i);
        }
    }
    Some(weights.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_interpolates_and_wraps() {
        let c = DensityCurve::new(vec![(24.0, 0.1), (0.0, 0.1), (6.0, 1.0)]);
        assert!((c.at(3.0) - 0.55).abs() < 1e-6);
        assert!((c.at(12.0) - 0.7).abs() < 0.31); // between 1.0 at 6 and 0.1 at 24
        assert!((c.at(24.0) - 0.1).abs() < 1e-6 && (c.at(48.0) - 0.1).abs() < 1e-6);
        assert_eq!(DensityCurve::default().at(5.0), 1.0);
    }

    #[test]
    fn group_density_follows_rule_then_default_chain() {
        // Berlin-Spandau: NormalCars 1, Trucks 0, Commercials 1, Ambulance 1, GDRCars 0.
        let defaults = [1, 0, 1, 1, 0];
        let mut lane = LaneDensity::default();
        assert_eq!(lane.for_group(0, &defaults), MEDIUM_DENSITY);
        assert_eq!(lane.for_group(1, &defaults), 0.0, "Trucks: default 0");
        assert_eq!(
            lane.for_group(2, &defaults),
            MEDIUM_DENSITY,
            "via NormalCars"
        );
        assert_eq!(lane.for_group(4, &defaults), 0.0, "GDRCars: default 0");
        assert_eq!(lane.for_group(7, &defaults), 0.0, "no such group");
        lane.set(0, 0.5);
        lane.set(4, 3.0);
        assert_eq!(
            lane.for_group(2, &defaults),
            0.5,
            "Commercials follow NormalCars' rule"
        );
        assert_eq!(lane.for_group(4, &defaults), 3.0, "unclamped");
        assert!(!lane.closed_for(4) && !lane.closed_for(1));
        lane.set(0, 0.0);
        assert!(lane.closed_for(0) && !lane.closed_for(1));
        // Only the rule of the named group counts; a later rule for it wins.
        lane.set(4, 1.0);
        assert_eq!(lane.iter().collect::<Vec<_>>(), vec![(0, 0.0), (4, 1.0)]);
        assert!(!lane.set(MAX_DENSITY_GROUPS, 1.0));
        // A cycle of defaults ends in Medium.
        assert_eq!(LaneDensity::default().for_group(0, &[2, 1]), MEDIUM_DENSITY);
    }

    #[test]
    fn targets_scale_proportionally_at_the_cap() {
        let (t, capped) = group_targets(&[30.0, 10.0, 0.0], 100);
        assert_eq!((t, capped), (vec![30.0, 10.0, 0.0], false));
        let (t, capped) = group_targets(&[150.0, 50.0], 100);
        assert!(capped);
        assert!((t[0] - 75.0).abs() < 1e-9 && (t[1] - 25.0).abs() < 1e-9);
        assert!((t[0] / t[1] - 3.0).abs() < 1e-9, "shares kept");
    }

    #[test]
    fn picks_follow_the_weights() {
        let mut rng = Rng::new(42);
        let w = [Some(7.0), Some(3.0), Some(0.0)];
        let mut counts = [0usize; 3];
        for _ in 0..10_000 {
            counts[weighted_pick(&w, &mut rng).unwrap()] += 1;
        }
        assert_eq!(counts[2], 0);
        let share = counts[0] as f64 / 10_000.0;
        assert!((share - 0.7).abs() < 0.03, "share {share}");
    }

    #[test]
    fn missing_weights_count_as_one_and_empty_lists_pick_nothing() {
        let mut rng = Rng::new(1);
        assert!(weighted_pick(&[None, None], &mut rng).is_some());
        assert_eq!(weighted_pick(&[], &mut rng), None);
        assert_eq!(weighted_pick(&[Some(0.0)], &mut rng), None);
    }
}
