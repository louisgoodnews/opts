//! `.cti` texture-change item files (repaints): `docs/src/formats/text/repaints.md`.
//!
//! A `.cti` holds `[item]` blocks (`<repaint name>`, `<[CTCTexture] slot>`, `<texture file>`) and
//! `[setvar]` blocks (`<script variable>`, `<value>`). Several `[item]` blocks with the same name
//! form one repaint; a `[setvar]` belongs to the repaint of the `[item]` that precedes it.

use super::text::{decode_bytes, scan_blocks};

/// One `[item]`: the texture a repaint puts into a `[CTCTexture]` slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtiItem {
    /// Repaint name (surrounding blanks removed).
    pub name: String,
    /// `[CTCTexture]` slot the texture replaces.
    pub slot: String,
    /// Texture file name, relative to the CTC folder.
    pub texture: String,
}

/// One `[setvar]`: a script variable set when its repaint is chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtiSetvar {
    /// Name of the repaint the line belongs to.
    pub repaint: String,
    /// Script variable.
    pub variable: String,
    /// Value as written.
    pub value: String,
}

/// Parsed content of one `.cti` file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CtiFile {
    /// `[item]` blocks in file order.
    pub items: Vec<CtiItem>,
    /// `[setvar]` blocks in file order.
    pub setvars: Vec<CtiSetvar>,
}

impl CtiFile {
    /// Parses a `.cti` file; blocks with missing parameters are skipped.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let mut out = Self::default();
        let mut current = String::new();
        for block in scan_blocks(&decode_bytes(bytes)) {
            match (block.keyword.as_str(), block.parameters.as_slice()) {
                ("item", [name, slot, texture, ..]) => {
                    current = name.trim().to_string();
                    out.items.push(CtiItem {
                        name: current.clone(),
                        slot: slot.trim().to_string(),
                        texture: texture.trim().to_string(),
                    });
                }
                ("setvar", [variable, value, ..]) if !current.is_empty() => {
                    out.setvars.push(CtiSetvar {
                        repaint: current.clone(),
                        variable: variable.trim().to_string(),
                        value: value.trim().to_string(),
                    });
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_group_by_name_and_setvars_follow_their_item() {
        let cti = CtiFile::parse(
            b"banner\n\n[item]\n Beige (AL)\nfarbschema_tex1\nEN92_1_AL.tga\n[item]\n Beige (AL)\nfarbschema_trans\ntr.tga\n[setvar]\nvis_wimpel_invisible\n1\n",
        );
        assert_eq!(cti.items.len(), 2);
        assert_eq!(cti.items[0].name, "Beige (AL)");
        assert_eq!(cti.items[1].slot, "farbschema_trans");
        assert_eq!(cti.setvars[0].repaint, "Beige (AL)");
        assert_eq!(cti.setvars[0].value, "1");
    }

    #[test]
    fn setvar_before_any_item_is_ignored() {
        assert!(CtiFile::parse(b"[setvar]\nx\n1\n").setvars.is_empty());
    }
}
