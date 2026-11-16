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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cold_start_at_five_degrees_puffs_and_a_warm_idle_at_thirty_does_not() {
        assert!(puff_rate_per_s(5.0, 0.0) > 0.0);
        assert_eq!(puff_rate_per_s(30.0, 0.0), 0.0);
    }

    #[test]
    fn heavy_load_shows_even_on_a_warm_day() {
        assert!(puff_rate_per_s(30.0, 0.9) > puff_rate_per_s(30.0, 0.3));
        assert_eq!(puff_rate_per_s(30.0, 1.0), MAX_PUFFS_PER_S);
    }

    #[test]
    fn the_cold_part_falls_linearly_and_is_bounded() {
        assert_eq!(cold_factor(COLD_BELOW_C), 0.0);
        assert_eq!(cold_factor(COLD_FULL_C - 5.0), 1.0);
        let mid = cold_factor((COLD_BELOW_C + COLD_FULL_C) / 2.0);
        assert!((mid - 0.5).abs() < 1e-6);
    }

    #[test]
    fn the_rate_is_capped_and_inputs_are_clamped() {
        assert_eq!(puff_rate_per_s(-30.0, 5.0), MAX_PUFFS_PER_S);
        assert_eq!(puff_rate_per_s(30.0, -1.0), 0.0);
    }
}
