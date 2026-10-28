//! Parser orchestration and field helpers for `.sco` scenery object files.

use super::*;

/// What the parser carries from one block to the next.
#[derive(Default)]
pub(super) struct ParseState {
    /// The distance of the last `[LOD]` block; the next `[mesh]` takes it.
    pub(super) current_lod: Option<f32>,
    /// `[use_traffic_light]` governs the next `[path]` only (every controlled path of the
    /// installed junctions has its own, repeated line; the uncontrolled approach stubs between
    /// them would otherwise inherit it).
    pub(super) active_traffic_light: Option<usize>,
    /// `[approachdist]` of the light that governs the next `[path]`.
    pub(super) active_approach_dist: Option<f32>,
    /// Whether a `[path]` was seen yet.
    pub(super) seen_path: bool,
    /// `[switchdir]` of the paths that follow, until the next one.
    pub(super) switch_position: Option<u32>,
}

impl SceneryObject {
    /// Parses a scenery object from raw file bytes.
    ///
    /// Automatically handles UTF-16LE/BE BOM, UTF-8 BOM, UTF-8, and CP1252 encodings.
    ///
    /// # Errors
    /// Returns [`SceneryParseError`] if decoding fails or numeric fields are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, SceneryParseError> {
        let text = decode_bytes(bytes);
        Self::from_text(&text)
    }

    /// Parses a scenery object from decoded text.
    ///
    /// # Errors
    /// Returns [`SceneryParseError`] if required parameters are missing or malformed.
    pub fn from_text(text: &str) -> Result<Self, SceneryParseError> {
        let blocks = scan_blocks(text);
        let mut obj = SceneryObject::default();
        let mut st = ParseState::default();

        for block in &blocks {
            let _handled = object::apply(&mut obj, &mut st, block)?
                || meshes::apply(&mut obj, &mut st, block)?
                || traffic::apply(&mut obj, &mut st, block)?
                || texts::apply(&mut obj, block)?;
        }

        // A path's approach distance comes from the light that governs it.
        let approach: Vec<Option<f32>> = obj
            .paths
            .iter()
            .map(|p| {
                p.traffic_light_index
                    .and_then(|i| obj.traffic_light(i))
                    .and_then(|(_, prog)| prog.approach_distance_m)
            })
            .collect();
        for (path, a) in obj.paths.iter_mut().zip(approach) {
            if a.is_some() {
                path.approach_distance_m = a;
            }
        }
        Ok(obj)
    }
}

pub(super) fn parse_counted_list(
    params: &[String],
    target: &mut Vec<String>,
    name: &'static str,
    line: usize,
) -> Result<(), SceneryParseError> {
    if let Some(first) = params.first() {
        let count = parse_usize(first, name, line)?;
        for item in params.iter().skip(1).take(count) {
            target.push(item.clone());
        }
    }
    Ok(())
}

/// The variable of a `[matl_lightmap]`: its second line when that is a name (not a number, a file
/// or a comment).
pub(super) fn light_map_variable(params: &[String]) -> Option<String> {
    let v = params.get(1)?.trim();
    let looks_like_name = v
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && !v.contains(' ')
        && !v.contains('.');
    looks_like_name.then(|| v.to_owned())
}

/// Parses a light effect block and records which mesh and LOD group it follows.
pub(super) fn add_light(obj: &mut SceneryObject, block: &OmsiBlock, lod: Option<f32>) {
    let parsed = if block.keyword == "light_enh" {
        parse_light_enh(block)
    } else {
        parse_light_enh_2(block)
    };
    match parsed {
        Ok(mut light) => {
            light.after_mesh = obj.meshes.len().checked_sub(1);
            light.lod_threshold = lod;
            obj.lights.push(light);
        }
        Err(e) => obj.warnings.push(e),
    }
}

/// The sub-keys of a `[new_attachment]` block: `attach_trans x y z` and `attach_rot_x|y|z angle`
/// (8,000 `attach_trans` and 1,700 `attach_rot_z` in the installed corpus). Unknown sub-keys and
/// unreadable numbers leave the value at zero: the point still exists, so the numbering of the
/// following points stays right.
pub(super) fn parse_attachment(params: &[String]) -> ScoAttachment {
    let mut a = ScoAttachment::default();
    let num = |i: usize| {
        params
            .get(i)
            .and_then(|s| parse_float(s).ok())
            .unwrap_or(0.0)
    };
    let mut i = 0;
    while i < params.len() {
        match params[i].trim().to_ascii_lowercase().as_str() {
            "attach_trans" => {
                a.translation = Vec3::new(num(i + 1), num(i + 2), num(i + 3));
                i += 4;
            }
            "attach_rot_x" => {
                a.rotation_deg.x = num(i + 1);
                i += 2;
            }
            "attach_rot_y" => {
                a.rotation_deg.y = num(i + 1);
                i += 2;
            }
            "attach_rot_z" => {
                a.rotation_deg.z = num(i + 1);
                i += 2;
            }
            _ => i += 1,
        }
    }
    a
}

pub(super) fn parse_f32(
    s: &str,
    field: &'static str,
    line: usize,
) -> Result<f32, SceneryParseError> {
    parse_float(s).map_err(|_| SceneryParseError::InvalidNumber {
        field,
        value: s.to_string(),
        line,
    })
}

pub(super) fn parse_u32(
    s: &str,
    field: &'static str,
    line: usize,
) -> Result<u32, SceneryParseError> {
    s.trim()
        .parse::<u32>()
        .map_err(|_| SceneryParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

pub(super) fn parse_u8(s: &str, field: &'static str, line: usize) -> Result<u8, SceneryParseError> {
    s.trim()
        .parse::<u8>()
        .map_err(|_| SceneryParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

pub(super) fn parse_usize(
    s: &str,
    field: &'static str,
    line: usize,
) -> Result<usize, SceneryParseError> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| SceneryParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}
