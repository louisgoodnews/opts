//! Weather files: presets (`Weather/*.owt`, scenario `*.osn.owt`), the cloud catalogue
//! (`Weather/clouds.cfg`) and the localised names and descriptions (`<Name>_<LANG>.dsc`).
//!
//! Layout and evidence: `docs/src/formats/text/environment.md`; use in the game:
//! `docs/src/architecture/weather.md`. The encoding differs per file (CP1252, UTF-8, UTF-16LE), so
//! every file goes through [`decode_bytes`].

use crate::formats::text::{decode_bytes, parse_float, scan_blocks_with, OmsiBlock};

mod folder;
mod parse;
mod types;

pub use folder::*;
pub use types::*;

/// Removes one pair of `<…>` around a name (`<Best Weather>` → `Best Weather`).
#[must_use]
pub fn strip_brackets(name: &str) -> &str {
    let n = name.trim();
    n.strip_prefix('<')
        .and_then(|n| n.strip_suffix('>'))
        .unwrap_or(n)
        .trim()
}

/// Parameter counts of the keywords, so the explanatory text between blocks is not taken for
/// values (`#CAVOK.owt` carries a long German comment between its blocks).
fn arity(keyword: &str) -> Option<usize> {
    match keyword {
        "name" | "press" => Some(1),
        "fog" | "wind" | "temp" | "clouds" => Some(2),
        "precip" => Some(5),
        "groundwet" => Some(3),
        "snow" | "snowonroad" | "end" => Some(0),
        _ => None,
    }
}

fn number(params: &[String], i: usize) -> Option<f32> {
    parse_float(params.get(i)?).ok().filter(|v| v.is_finite())
}

fn pair(block: &OmsiBlock) -> Option<(f32, f32)> {
    Some((number(&block.parameters, 0)?, number(&block.parameters, 1)?))
}

#[cfg(test)]
mod tests;
