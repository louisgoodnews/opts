//! The cloud catalogue of a `Weather` folder (`clouds.cfg`): cover classes and cloud types.

use super::*;

/// How a cloud layer is drawn (`clouds.cfg`, last line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudCover {
    /// `sct`: scattered clouds with gaps.
    Scattered,
    /// `ovc`: a closed overcast layer.
    Overcast,
}

/// One `[cloudtype]` of `clouds.cfg`.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudType {
    /// Name (`Cumulus 1`, `Overcast 1`), matched case-insensitively by `[clouds]`.
    pub name: String,
    /// Texture file below `Texture/`.
    pub texture: String,
    /// Edge length of one texture repeat in metres.
    pub size_m: f32,
    /// Cover class.
    pub cover: CloudCover,
}

/// The cloud catalogue (`Weather/clouds.cfg`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CloudTypes(pub Vec<CloudType>);

impl CloudTypes {
    /// Parses `clouds.cfg`: `[cloudtype]` blocks of four lines (name, texture, size in metres,
    /// `sct` or `ovc`); the German explanation at the top is skipped. A block with a bad number
    /// or class is dropped.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let blocks = scan_blocks_with(&decode_bytes(bytes), |k| (k == "cloudtype").then_some(4));
        Self(
            blocks
                .iter()
                .filter(|b| b.keyword == "cloudtype")
                .filter_map(|b| {
                    let p = &b.parameters;
                    let cover = match p.get(3)?.trim().to_ascii_lowercase().as_str() {
                        "sct" => CloudCover::Scattered,
                        "ovc" => CloudCover::Overcast,
                        _ => return None,
                    };
                    Some(CloudType {
                        name: p.first()?.trim().to_owned(),
                        texture: p.get(1)?.trim().to_owned(),
                        size_m: number(p, 2).filter(|s| *s > 0.0)?,
                        cover,
                    })
                })
                .collect(),
        )
    }

    /// The type with this name (case-insensitive).
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&CloudType> {
        self.0
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name.trim()))
    }
}
