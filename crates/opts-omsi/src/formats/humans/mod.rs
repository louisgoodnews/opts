//! Parser for OMSI human configuration files (`.hum`).
//!
//! A `.hum` is an anthropometric and kinematic profile: it names the model manifest that
//! carries the skinned mesh, and gives stature, seat height, joint pivots and gait
//! parameters. See `docs/src/formats/text/humans.md`.

pub mod error;

use glam::{Vec2, Vec3};

pub use error::HumanParseError;

use crate::formats::text::{decode_bytes, parse_float, scan_blocks, OmsiBlock};

/// Distance from the crown of the head to the eyes in metres (`ego-perspective.md`).
pub const EYE_BELOW_CROWN_M: f32 = 0.11;

/// Right-side joint pivots from `[links]`, in the character's local frame
/// (`x` lateral, `y` longitudinal, `z` vertical; origin on the ground between the feet).
/// The engine mirrors them across `x = 0` for the left side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HumanLinks {
    /// Right hip `(x, y, z)`.
    pub hip: Vec3,
    /// Right knee `(x, y, z)`.
    pub knee: Vec3,
    /// Waist on the midline `(y, z)`.
    pub waist: Vec2,
    /// Right shoulder `(x, y, z)`.
    pub shoulder: Vec3,
    /// Right elbow `(x, y, z)`.
    pub elbow: Vec3,
    /// Neck base on the midline `(y, z)`.
    pub neck: Vec2,
    /// Right wrist / carpus `(x, y, z)`.
    pub hand: Vec3,
    /// Right fingertip `(x, y, z)`.
    pub finger: Vec3,
}

/// Gait parameters from `[walk_param]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkParams {
    /// Stride length of a full double step in metres (`schrittweite`).
    pub stride_m: f32,
    /// Resting outward flare of the upper arm in degrees (`upper_arm_beta`).
    pub upper_arm_beta_deg: f32,
    /// Arm swing multiplier (`armschwing`).
    pub arm_swing: f32,
    /// Hip lateral oscillation multiplier (`walk_huefte`).
    pub hip_sway: f32,
    /// Waist lateral sway multiplier (`anim_walk_taille`).
    pub waist_sway: f32,
}

/// A parsed `.hum` file.
#[derive(Debug, Clone, PartialEq)]
pub struct HumanProfile {
    /// Model manifest path, relative to the `.hum` file's directory (`[model]`).
    pub model: String,
    /// Seated hip height above the floor in metres (`[seatheight]`).
    pub seat_height_m: f32,
    /// Lateral distance between the feet in metres (`[humangeom]` line 1).
    pub feet_dist_m: f32,
    /// Standing stature in metres (`[humangeom]` line 2).
    pub height_m: f32,
    /// Joint pivots (`[links]`).
    pub links: HumanLinks,
    /// Gait parameters (`[walk_param]`).
    pub walk: WalkParams,
    /// Voice profile id such as `M1`, `W2`, `D1` (`[voice]`).
    pub voice: String,
    /// Body mass in tonnes (`[mass]`).
    pub mass_tonnes: f32,
    /// Age in years, if declared (`[age]`).
    pub age_years: Option<u32>,
}

impl HumanProfile {
    /// Parses a `.hum` file from raw bytes (CP1252, UTF-8 or UTF-16).
    ///
    /// # Errors
    /// Returns [`HumanParseError`] if a mandatory keyword is missing or a value is malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, HumanParseError> {
        Self::from_text(&decode_bytes(bytes))
    }

    /// Parses a `.hum` file from decoded text.
    ///
    /// Template prose and `{name}` placeholders that authors leave between blocks are ignored
    /// because only the first N parameter lines of each keyword are read.
    ///
    /// # Errors
    /// Returns [`HumanParseError`] if a mandatory keyword is missing or a value is malformed.
    pub fn from_text(text: &str) -> Result<Self, HumanParseError> {
        let blocks = scan_blocks(text);
        // Authoring templates contain an indented, therefore inert, example of each keyword
        // before the real one. The real block is the last one with numeric first parameters.
        let find = |kw: &'static str| -> Option<&OmsiBlock> {
            blocks
                .iter()
                .filter(|b| b.keyword == kw)
                .rfind(|b| b.parameters.first().is_none_or(|p| !p.starts_with('{')))
        };
        let req = |kw: &'static str| find(kw).ok_or(HumanParseError::MissingKeyword(kw));
        let floats =
            |b: &OmsiBlock, kw: &'static str, n: usize| -> Result<Vec<f32>, HumanParseError> {
                if b.parameters.len() < n {
                    return Err(HumanParseError::TooFewParameters {
                        keyword: kw,
                        expected: n,
                        found: b.parameters.len(),
                        line: b.line_number,
                    });
                }
                b.parameters[..n]
                    .iter()
                    .map(|p| {
                        parse_float(p).map_err(|_| HumanParseError::InvalidNumber {
                            keyword: kw,
                            value: p.clone(),
                            line: b.line_number,
                        })
                    })
                    .collect()
            };
        let text_param = |b: &OmsiBlock, kw: &'static str| -> Result<String, HumanParseError> {
            b.parameters.first().map(|s| s.trim().to_string()).ok_or(
                HumanParseError::TooFewParameters {
                    keyword: kw,
                    expected: 1,
                    found: 0,
                    line: b.line_number,
                },
            )
        };

        let model = text_param(req("model")?, "model")?;
        let seat_height_m = floats(req("seatheight")?, "seatheight", 1)?[0];
        let geom = floats(req("humangeom")?, "humangeom", 2)?;
        let l = floats(req("links")?, "links", 22)?;
        let w = floats(req("walk_param")?, "walk_param", 5)?;
        let voice = text_param(req("voice")?, "voice")?;
        let mass_tonnes = floats(req("mass")?, "mass", 1)?[0];
        let age_years = match find("age") {
            Some(b) => Some(floats(b, "age", 1)?[0].round().max(0.0) as u32),
            None => None,
        };

        Ok(Self {
            model,
            seat_height_m,
            feet_dist_m: geom[0],
            height_m: geom[1],
            links: HumanLinks {
                hip: Vec3::new(l[0], l[1], l[2]),
                knee: Vec3::new(l[3], l[4], l[5]),
                waist: Vec2::new(l[6], l[7]),
                shoulder: Vec3::new(l[8], l[9], l[10]),
                elbow: Vec3::new(l[11], l[12], l[13]),
                neck: Vec2::new(l[14], l[15]),
                hand: Vec3::new(l[16], l[17], l[18]),
                finger: Vec3::new(l[19], l[20], l[21]),
            },
            walk: WalkParams {
                stride_m: w[0],
                upper_arm_beta_deg: w[1],
                arm_swing: w[2],
                hip_sway: w[3],
                waist_sway: w[4],
            },
            voice,
            mass_tonnes,
            age_years,
        })
    }

    /// Eye position in the character's local frame `(x, y, z)`: on the midline, above the neck
    /// base's longitudinal offset, [`EYE_BELOW_CROWN_M`] below the crown.
    #[must_use]
    pub fn eye_position(&self) -> Vec3 {
        Vec3::new(0.0, self.links.neck.x, self.height_m - EYE_BELOW_CROWN_M)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[model]\nmodel\\man.cfg\n\n[seatheight]\n0.81\n\n\t[humangeom]\n\t{feetdist (FD)}\n\t{height}\n\n\t[links]\n\t{x_hip}\n\n[humangeom]\n0.04\n1.77\n\n[links]\n0.09\n0.00\n0.92\n0.09\n-0.03\n0.53\n0.02\n1.17\n0.18\n-0.05\n1.43\n0.44\n-0.04\n1.41\n-0.02\n1.55\n0.69\n-0.03\n1.43\n0.90\n-0.03\n1.43\n\n[walk_param]\n1.4\n70\n0.7\n1.0\n0.0\n\n[voice]\nM2\n\n[mass]\n0.07\n\n[age]\n12\n";

    #[test]
    fn parses_template_commented_profile() {
        let h = HumanProfile::from_text(SAMPLE).expect("parse");
        assert_eq!(h.model, "model\\man.cfg");
        assert_eq!(
            (h.seat_height_m, h.feet_dist_m, h.height_m),
            (0.81, 0.04, 1.77)
        );
        assert_eq!(h.links.hip, Vec3::new(0.09, 0.0, 0.92));
        assert_eq!(h.links.neck, Vec2::new(-0.02, 1.55));
        assert_eq!(h.links.finger, Vec3::new(0.9, -0.03, 1.43));
        assert_eq!(h.walk.stride_m, 1.4);
        assert_eq!(
            (h.voice.as_str(), h.mass_tonnes, h.age_years),
            ("M2", 0.07, Some(12))
        );
        assert!((h.eye_position().z - 1.66).abs() < 1e-5);
    }

    #[test]
    fn reports_missing_and_malformed_fields() {
        assert_eq!(
            HumanProfile::from_text("[model]\na.cfg\n"),
            Err(HumanParseError::MissingKeyword("seatheight"))
        );
        let broken = SAMPLE.replace("0.81", "tall");
        assert!(matches!(
            HumanProfile::from_text(&broken),
            Err(HumanParseError::InvalidNumber {
                keyword: "seatheight",
                ..
            })
        ));
        let short = SAMPLE.replace("[mass]\n0.07\n", "[mass]\n");
        assert!(matches!(
            HumanProfile::from_text(&short),
            Err(HumanParseError::TooFewParameters {
                keyword: "mass",
                ..
            })
        ));
    }
}
