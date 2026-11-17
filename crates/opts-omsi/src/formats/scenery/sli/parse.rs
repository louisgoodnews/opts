//! Block parser for OMSI spline profile (`.sli`) files.

use super::*;
use crate::formats::text::OmsiBlock;

mod meta;
mod profile;
mod scalars;

use meta::{handle_meta, handle_texture};
use profile::{handle_path, handle_profile, handle_rail_enh};
use scalars::{parse_f32, parse_u32, parse_usize};

impl SplineProfile {
    /// Parses a spline profile from raw file bytes.
    ///
    /// Automatically handles UTF-16LE/BE BOM, UTF-8 BOM, UTF-8, and CP1252 encodings.
    ///
    /// # Errors
    /// Returns [`SceneryParseError`] if decoding fails or numeric fields are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, SceneryParseError> {
        let text = decode_bytes(bytes);
        Self::from_text(&text)
    }

    /// Parses a spline profile from decoded text.
    ///
    /// A block with a malformed value is skipped with a warning; the rest of the file is kept,
    /// so one stray line (a comment or a tutorial fragment inside a block) does not drop every
    /// profile point of the spline.
    ///
    /// # Errors
    /// Never fails for malformed blocks; the `Result` is kept for the reader's signature.
    pub fn from_text(text: &str) -> Result<Self, SceneryParseError> {
        let blocks = scan_blocks(text);
        let mut sli = SplineProfile::default();

        for block in &blocks {
            if let Err(e) = apply_block(block, &mut sli) {
                tracing::warn!(
                    line = block.line_number,
                    keyword = %block.keyword,
                    error = %e,
                    "skipping malformed spline block"
                );
            }
        }

        Ok(sli)
    }
}

fn apply_block(block: &OmsiBlock, sli: &mut SplineProfile) -> Result<(), SceneryParseError> {
    match block.keyword.as_str() {
        "length" | "halfcantwidth" | "scaletexbylength" | "rendertype" | "onlyeditor"
        | "heightprofile" | "heightproflile" | "heigthprofile" => handle_meta(block, sli),
        "texture" | "matl" | "matl_alpha" => handle_texture(block, sli),
        "profile" | "profilepnt" => handle_profile(block, sli),
        "path" | "path_2" => handle_path(block, sli),
        "rail_enh" => handle_rail_enh(block, sli),
        _ => Ok(()),
    }
}
