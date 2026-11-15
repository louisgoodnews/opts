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
    /// # Errors
    /// Returns [`SceneryParseError`] if required parameters are missing or malformed.
    pub fn from_text(text: &str) -> Result<Self, SceneryParseError> {
        let blocks = scan_blocks(text);
        let mut sli = SplineProfile::default();

        for block in &blocks {
            match block.keyword.as_str() {
                "length" | "halfcantwidth" | "scaletexbylength" | "rendertype" | "onlyeditor"
                | "heightprofile" | "heightproflile" | "heigthprofile" => {
                    handle_meta(block, &mut sli)?;
                }
                "texture" | "matl" | "matl_alpha" => {
                    handle_texture(block, &mut sli)?;
                }
                "profile" | "profilepnt" => {
                    handle_profile(block, &mut sli)?;
                }
                "path" | "path_2" => {
                    handle_path(block, &mut sli)?;
                }
                "rail_enh" => {
                    handle_rail_enh(block, &mut sli)?;
                }
                _ => {}
            }
        }

        Ok(sli)
    }
}
