//! Exhaust plumes (M-0117): how many puffs a bus puts out per second, from the ambient air
//! temperature and the engine load. Cold starts and heavy acceleration show; a warm engine at
//! idle or the cruise of a hot day does not. Pure numbers: the host draws the particles.
//!
//! The rule is a design choice (❓ `docs/src/architecture/weather.md`, section 10): the cold part
//! and the load part add up, and the total caps at [`MAX_PUFFS_PER_S`].

/// Air temperature (°C) below which the engine's exhaust shows at idle (cold start, cold air).
pub const COLD_BELOW_C: f32 = 15.0;
/// Air temperature (°C) of the full cold part: at and below it the cold part is 1.
pub const COLD_FULL_C: f32 = -10.0;
/// Puffs per second at full effect (the host's particle budget per vehicle).
pub const MAX_PUFFS_PER_S: f32 = 6.0;
/// Seconds a puff stays visible before the host removes it.
pub const PUFF_LIFETIME_S: f32 = 1.5;

/// The cold part of the plume, 0 (at or above [`COLD_BELOW_C`]) to 1 (at or below
/// [`COLD_FULL_C`]), linear in between.
#[must_use]
pub fn cold_factor(ambient_c: f32) -> f32 {
    ((COLD_BELOW_C - ambient_c) / (COLD_BELOW_C - COLD_FULL_C)).clamp(0.0, 1.0)
}

/// Puffs per second for the air temperature `ambient_c` (°C) and the engine load `load`
/// (0 idle to 1 full throttle; clamped). 0 on a warm engine at idle.
#[must_use]
pub fn puff_rate_per_s(ambient_c: f32, load: f32) -> f32 {
    let effect = (cold_factor(ambient_c) + load.clamp(0.0, 1.0)).min(1.0);
    MAX_PUFFS_PER_S * effect
}

/// Puffs to start in a frame of `dt_s` seconds at `rate_per_s`, with `live` puffs still visible and
/// at most `cap` of them at a time. Returns the count and the fraction to carry into the next
/// frame (at most one puff, so a capped emitter does not save up a burst).
#[must_use]
pub fn puffs_to_spawn(rate_per_s: f32, dt_s: f32, live: u32, cap: u32, carry: f32) -> (u32, f32) {
    let room = cap.saturating_sub(live);
    let due = carry + rate_per_s.max(0.0) * dt_s.max(0.0);
    let whole = (due.floor().max(0.0) as u32).min(room);
    (whole, (due - whole as f32).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests;
