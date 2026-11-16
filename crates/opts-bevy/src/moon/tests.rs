//! Unit tests for `super` (`moon.rs`).

use super::*;

/// Julian day of a UTC date and time.
fn jd_of(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> f64 {
    julian_day(
        CalendarDate::new(y, mo, d),
        f64::from(h) * 3600.0 + f64::from(mi) * 60.0,
    )
}

/// Solves for the instant near `reference` at which the elongation passes `target` degrees.
fn solve(reference: f64, target: f64) -> f64 {
    let f = |jd: f64| {
        let mut d = lunar_elongation_deg(jd) - target;
        while d > 180.0 {
            d -= 360.0;
        }
        while d < -180.0 {
            d += 360.0;
        }
        d
    };
    let (mut lo, mut hi) = (reference - 1.0, reference + 1.0);
    assert!(f(lo) < 0.0 && f(hi) > 0.0, "bracket");
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

mod light;
mod phases;
mod sky;

