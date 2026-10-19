//! Light effects of scenery objects: `[light_enh]` and `[light_enh_2]`.
//!
//! Layouts (`docs/src/formats/text/sco.md`, "Lights & Effects"):
//!
//! * `[light_enh]`: x, y, z, r, g, b, size, *variable*, then tuning values;
//! * `[light_enh_2]`: position (3), direction (3), up vector (3), omni, rotating, r, g, b, size,
//!   inner cone angle, outer cone angle, *variable*, factor, z-offset, parameter bits, cone,
//!   time constant and an optional bitmap (23 or 24 values).
//!
//! Positions and directions are object-local OMSI axes (x right, y forward, z up). Colours are
//! 0–255. A block whose numbers do not parse is reported and skipped, never fatal.

use glam::Vec3;

use crate::formats::text::{parse_float, OmsiBlock};

/// One light effect (corona) of a scenery object.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoLight {
    /// Position in object-local metres.
    pub position: Vec3,
    /// Direction the light shines to (unit or zero; `[light_enh]` has none).
    pub direction: Vec3,
    /// Colour, 0–255 per channel.
    pub color: [f32; 3],
    /// Size of the effect in metres.
    pub size_m: f32,
    /// Brightness variable: `0` dark, `1` normal, `2` double. Empty means always `1`.
    pub variable: String,
    /// Multiplier on the variable.
    pub factor: f32,
    /// `true` for omnidirectional lights (`[light_enh]` is always omnidirectional).
    pub omni: bool,
    /// Angle of full brightness around `direction` in degrees.
    pub cone_inner_deg: f32,
    /// Angle at which the light becomes visible in degrees.
    pub cone_outer_deg: f32,
    /// Seconds to reach 63 % brightness (0 = immediate).
    pub time_const_s: f32,
    /// The `cone` flag of `[light_enh_2]` (parameter 22 = `1`): the light also draws a volumetric
    /// light cone (not for omnidirectional lights). `false` for `[light_enh]`.
    pub volumetric_cone: bool,
    /// Effect texture; empty for the default.
    pub bitmap: String,
    /// Index of the `[mesh]` declared before the light (`None` when it precedes every mesh).
    pub after_mesh: Option<usize>,
    /// `[LOD]` threshold in force where the light is declared.
    pub lod_threshold: Option<f32>,
}

fn num(params: &[String], i: usize) -> Option<f32> {
    params.get(i).and_then(|s| parse_float(s).ok())
}

fn vec3(params: &[String], i: usize) -> Option<Vec3> {
    Some(Vec3::new(
        num(params, i)?,
        num(params, i + 1)?,
        num(params, i + 2)?,
    ))
}

/// Parses `[light_enh]`; `Err` names the problem.
pub fn parse_light_enh(block: &OmsiBlock) -> Result<ScoLight, String> {
    let p = &block.parameters;
    let bad = || format!("line {}: malformed [light_enh]", block.line_number);
    let position = vec3(p, 0).ok_or_else(bad)?;
    let color = [
        num(p, 3).ok_or_else(bad)?,
        num(p, 4).ok_or_else(bad)?,
        num(p, 5).ok_or_else(bad)?,
    ];
    let size_m = num(p, 6).ok_or_else(bad)?;
    let variable = p.get(7).cloned().unwrap_or_default();
    Ok(ScoLight {
        position,
        direction: Vec3::ZERO,
        color,
        size_m,
        variable,
        factor: 1.0,
        omni: true,
        cone_inner_deg: 180.0,
        cone_outer_deg: 180.0,
        time_const_s: 0.0,
        volumetric_cone: false,
        bitmap: String::new(),
        after_mesh: None,
        lod_threshold: None,
    })
}

/// Parses `[light_enh_2]`; `Err` names the problem.
pub fn parse_light_enh_2(block: &OmsiBlock) -> Result<ScoLight, String> {
    let p = &block.parameters;
    let bad = || format!("line {}: malformed [light_enh_2]", block.line_number);
    let position = vec3(p, 0).ok_or_else(bad)?;
    let direction = vec3(p, 3).ok_or_else(bad)?.normalize_or_zero();
    let omni = num(p, 9).ok_or_else(bad)? != 0.0;
    let color = [
        num(p, 11).ok_or_else(bad)?,
        num(p, 12).ok_or_else(bad)?,
        num(p, 13).ok_or_else(bad)?,
    ];
    let size_m = num(p, 14).ok_or_else(bad)?;
    let cone_inner_deg = num(p, 15).ok_or_else(bad)?;
    let cone_outer_deg = num(p, 16).ok_or_else(bad)?;
    let variable = p.get(17).cloned().ok_or_else(bad)?;
    let factor = num(p, 18).unwrap_or(1.0);
    let time_const_s = num(p, 22).unwrap_or(0.0);
    let volumetric_cone = num(p, 21).is_some_and(|v| (v - 1.0).abs() < 0.5);
    // The 24th value is an optional file name; free comment lines may follow, so only a value
    // that looks like an image file counts.
    let bitmap = p
        .get(23)
        .filter(|s| {
            let l = s.to_ascii_lowercase();
            [".bmp", ".tga", ".dds", ".png", ".jpg"]
                .iter()
                .any(|e| l.ends_with(e))
        })
        .cloned()
        .unwrap_or_default();
    Ok(ScoLight {
        position,
        direction,
        color,
        size_m,
        variable,
        factor,
        omni,
        cone_inner_deg,
        cone_outer_deg,
        time_const_s,
        volumetric_cone,
        bitmap,
        after_mesh: None,
        lod_threshold: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(keyword: &str, lines: &[&str]) -> OmsiBlock {
        OmsiBlock {
            keyword: keyword.into(),
            line_number: 1,
            parameters: lines.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn light_enh_2_reads_position_colour_cone_and_variable() {
        let b = block(
            "light_enh_2",
            &[
                "0", "-0.1", "0.3", "0", "2", "0", "0", "0", "1", "0", "0", "255", "10", "0",
                "0.25", "20", "50", "lamp_red", "1.5", "0.1", "1", "0", "0.1",
            ],
        );
        let l = parse_light_enh_2(&b).unwrap();
        assert_eq!(l.position, Vec3::new(0.0, -0.1, 0.3));
        assert_eq!(l.direction, Vec3::Y);
        assert!(!l.omni);
        assert_eq!(l.color, [255.0, 10.0, 0.0]);
        assert_eq!(l.size_m, 0.25);
        assert_eq!((l.cone_inner_deg, l.cone_outer_deg), (20.0, 50.0));
        assert_eq!(l.variable, "lamp_red");
        assert_eq!(l.factor, 1.5);
        assert_eq!(l.time_const_s, 0.1);
        assert!(l.bitmap.is_empty());
        assert!(!l.volumetric_cone);
    }

    /// Parameter 22 (`cone` = 1) marks a light that draws a volumetric cone; the rest do not.
    #[test]
    fn light_enh_2_reads_the_volumetric_cone_flag() {
        let mut v: Vec<String> = [
            "0",
            "0",
            "7",
            "0",
            "0",
            "-1",
            "0",
            "1",
            "0",
            "0",
            "1",
            "230",
            "230",
            "255",
            "2",
            "120",
            "200",
            "NightlightA",
            "0.8",
            "0.5",
            "1",
            "1",
            "0.2",
        ]
        .map(String::from)
        .to_vec();
        let mk = |v: &[String]| OmsiBlock {
            keyword: "light_enh_2".into(),
            line_number: 1,
            parameters: v.to_vec(),
        };
        assert!(parse_light_enh_2(&mk(&v)).unwrap().volumetric_cone);
        v[21] = "0".into();
        assert!(!parse_light_enh_2(&mk(&v)).unwrap().volumetric_cone);
        v[21] = "2".into();
        assert!(!parse_light_enh_2(&mk(&v)).unwrap().volumetric_cone);
    }

    #[test]
    fn light_enh_reads_the_short_layout() {
        let b = block(
            "light_enh",
            &[
                "1", "2", "3", "255", "255", "200", "0.5", "night", "0", "0", "0", "0",
            ],
        );
        let l = parse_light_enh(&b).unwrap();
        assert_eq!(l.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(l.variable, "night");
        assert!(l.omni);
    }

    #[test]
    fn malformed_lights_are_errors_not_panics() {
        assert!(parse_light_enh_2(&block("light_enh_2", &["x"])).is_err());
        assert!(parse_light_enh(&block("light_enh", &[])).is_err());
    }
}
