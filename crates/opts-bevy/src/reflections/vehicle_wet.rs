//! Vehicle wetness as two parameters (M-0121): the paint and the tyres. The paint takes the
//! `[moisture]` rule of its texture ([`material_wetness`], the same as scenery). The tyres take a
//! share of the global wetness whatever their texture's flags say, because a tyre is wet from the
//! road, not from its paint. ❓ Assumed: no OMSI file says which texture is a tyre, so the names
//! are matched (see [`is_tyre_texture`]) and the share is a design value, not a measured one.

use super::wet::material_wetness;

/// The share of the global wetness a tyre takes (❓ design value, unverified on screen).
pub const TYRE_WET_SHARE: f32 = 0.5;

/// Whether a texture name is a tyre's: a word of the name starts with `reif` (German *Reifen*),
/// `tyre`, `tire` or `rubber`, in any case. ❓ Name matching; no file marks tyres.
#[must_use]
pub fn is_tyre_texture(name: &str) -> bool {
    name.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| {
            ["reif", "tyre", "tire", "rubber"]
                .iter()
                .any(|stem| word.starts_with(stem))
        })
}

/// The wetness a vehicle part takes at the global wetness `w` (0..=1). A tyre takes
/// [`TYRE_WET_SHARE`] of it whatever its flags; any other part takes the [`material_wetness`]
/// rule (the full value with `[moisture]`, dry without).
#[must_use]
pub fn part_wetness(tyre: bool, moisture: bool, w: f32) -> f32 {
    if tyre {
        w.clamp(0.0, 1.0) * TYRE_WET_SHARE
    } else {
        material_wetness(moisture, w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tyre_textures_are_recognised_by_their_words() {
        for name in [
            "Reifen_vorn.bmp",
            "TYRE01.dds",
            "tire_rear.bmp",
            "rubber.tga",
        ] {
            assert!(is_tyre_texture(name), "{name} is a tyre");
        }
        for name in ["Bus_Body.bmp", "entire_front.bmp", "glass.bmp", ""] {
            assert!(!is_tyre_texture(name), "{name} is not a tyre");
        }
    }

    #[test]
    fn a_tyre_takes_its_share_of_the_wetness_whatever_the_flag() {
        assert_eq!(part_wetness(true, false, 0.0), 0.0);
        assert_eq!(part_wetness(true, false, 1.0), TYRE_WET_SHARE);
        assert_eq!(part_wetness(true, true, 1.0), TYRE_WET_SHARE);
        assert_eq!(part_wetness(true, false, 4.0), TYRE_WET_SHARE);
    }

    #[test]
    fn paint_takes_the_moisture_rule_and_stays_dry_without_the_flag() {
        assert_eq!(part_wetness(false, true, 0.7), 0.7);
        assert_eq!(part_wetness(false, true, 3.0), 1.0);
        assert_eq!(part_wetness(false, false, 1.0), 0.0);
    }
}
