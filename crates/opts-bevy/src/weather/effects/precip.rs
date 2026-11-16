//! The precipitation drawn around the camera: its kind, quads, fall speed and drift.

use super::*;

/// Rain or snow around the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrecipPlan {
    /// Rain or snow.
    pub kind: PrecipKind,
    /// Quads in use, at most the quality's budget.
    pub quads: u32,
    /// Fall speed in m/s.
    pub fall_speed: f32,
    /// Horizontal velocity in m/s (the wind, damped).
    pub drift: Vec2,
    /// Width and length of a quad in metres.
    pub size: Vec2,
}

/// The precipitation to draw: none for dry weather, a budget of `0` or a tiny rate.
#[must_use]
pub fn precip_plan(c: &WeatherConditions, budget: u32) -> Option<PrecipPlan> {
    if c.precip == PrecipKind::None || c.precip_rate <= 0.0 || budget == 0 {
        return None;
    }
    let share = 0.15 + 0.85 * c.precip_rate;
    let quads = ((budget as f32 * share).round() as u32).clamp(1, budget);
    let (fall, damp, size) = match c.precip {
        PrecipKind::Rain => (9.0, 0.6, Vec2::new(0.025, 0.8)),
        _ => (1.4, 0.9, Vec2::new(0.08, 0.08)),
    };
    let mut drift = c.wind_to() * c.wind_speed * damp;
    let cap = fall * 1.5;
    if drift.length() > cap {
        drift = drift.normalize() * cap;
    }
    Some(PrecipPlan {
        kind: c.precip,
        quads,
        fall_speed: fall,
        drift,
        size,
    })
}
