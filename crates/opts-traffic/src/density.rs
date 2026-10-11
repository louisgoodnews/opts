//! Spawn weights and traffic density helpers.
//!
//! The data comes from `ailists.cfg` (weights) and `unsched_trafficdens.txt` (curves), parsed in
//! `opts_core::formats::map`. How OMSI turns a density factor into vehicles per kilometre is not
//! documented ❓; hosts choose their own scale (`docs/src/architecture/ai-traffic.md`).

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
