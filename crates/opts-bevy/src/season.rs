//! Season state of the scenery: the half-year variables (`winterhalbjahr`, `sommerhalbjahr`)
//! and the snow cover that selects `WinterSnow` texture variants.
//!
//! The rules are OPTS's own ❓: the stock season scripts read more than the month, so these are
//! the values the host supplies, not a copy of their logic.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// `winterhalbjahr` is 1 from October to March, `sommerhalbjahr` is its complement.
#[must_use]
pub fn winter_half_year(month: u32) -> bool {
    matches!(month, 1..=3 | 10..=12)
}

/// Snowing weather (`PrecipType` 2) is the snow the scenery shows (`snow_visible`).
#[must_use]
pub fn snows(precip_type: f32) -> bool {
    (precip_type - 2.0).abs() < 0.5
}

static SNOW_COVER: AtomicBool = AtomicBool::new(false);

/// Records whether the session has snow cover. The game sets it from the session weather each
/// frame; textures loaded afterwards pick their `WinterSnow` variants (already loaded objects
/// keep the textures they had).
pub fn set_snow_cover(on: bool) {
    SNOW_COVER.store(on, Ordering::Relaxed);
}

/// Whether the session currently has snow cover.
#[must_use]
pub fn snow_cover() -> bool {
    SNOW_COVER.load(Ordering::Relaxed)
}

/// Season folders searched before the base folders of a texture, in precedence order. Under snow
/// cover that is `WinterSnowfall` then `WinterSnow` inside each texture folder; otherwise none.
/// A variant must have the same name and extension as the base texture (`textures.md`), so a
/// missing variant falls back to the base texture, never to a missing one.
#[must_use]
pub fn variant_folders(base: &[PathBuf], snow: bool) -> Vec<PathBuf> {
    if !snow {
        return Vec::new();
    }
    base.iter()
        .flat_map(|d| ["WinterSnowfall", "WinterSnow"].map(|s| d.join(s)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winter_is_october_to_march() {
        assert!(winter_half_year(1) && winter_half_year(3) && winter_half_year(11));
        assert!(!winter_half_year(4) && !winter_half_year(9));
    }

    #[test]
    fn snow_is_the_snowing_precipitation_type() {
        assert!(snows(2.0));
        assert!(!snows(0.0) && !snows(1.0));
    }

    #[test]
    fn snow_variants_come_first_only_under_snow_cover() {
        let base = [PathBuf::from("obj/texture")];
        assert!(variant_folders(&base, false).is_empty());
        let v = variant_folders(&base, true);
        assert_eq!(v[0], PathBuf::from("obj/texture/WinterSnowfall"));
        assert_eq!(v[1], PathBuf::from("obj/texture/WinterSnow"));
    }

    #[test]
    fn the_snow_cover_flag_round_trips() {
        set_snow_cover(true);
        assert!(snow_cover());
        set_snow_cover(false);
        assert!(!snow_cover());
    }
}
