//! Preflight: parse the object with `opts-core`, collect its keywords, feature tags and context
//! classes, and decide which string variables the generated tile carries.

use std::collections::BTreeSet;
use std::path::PathBuf;

use opts_bevy::OmsiInstall;
use opts_core::formats::scenery::SceneryObject;
use opts_core::formats::text::{decode_bytes, scan_blocks};

use super::ObjectEntry;

/// Keywords OPTS parses but does not implement for scenery (or does not parse at all). They
/// become `unsupported` tags, never failures.
pub const UNSUPPORTED_KEYWORDS: &[&str] = &[
    "light_enh",
    "light_enh_2",
    "newanim",
    "visible",
    "maplight",
    "matl_lightmap",
    "matl_nightmap",
    "matl_envmap",
    "matl_transmap",
    "matl_change",
    "ctc",
    "alphascale",
    "lod",
    "splinehelper",
    "crossing_heightdeformation",
    "script",
    "mouseevent",
    "texttexture_enh",
    "particle_emitter",
    "smoke",
];

/// Which string variables a placement carries.
#[derive(Debug, Clone, PartialEq)]
pub enum StringPlan {
    /// None.
    None,
    /// `n` text-texture strings, from set A or B.
    Text(usize),
    /// A `[tree]`: texture, height and width ratio (the middle of the `.sco` ranges).
    Tree {
        /// Texture file.
        texture: String,
        /// Height in metres.
        height: f32,
        /// Width / height.
        ratio: f32,
    },
}

impl StringPlan {
    /// The string variables for string set `set` (`'A'` or `'B'`). Never empty strings.
    #[must_use]
    pub fn strings(&self, set: char) -> Vec<String> {
        match self {
            Self::None => Vec::new(),
            Self::Text(n) => (1..=*n)
                .map(|i| {
                    if set == 'A' {
                        format!("OPTS-A{i}")
                    } else {
                        format!("WWWW-B{i}")
                    }
                })
                .collect(),
            Self::Tree {
                texture,
                height,
                ratio,
            } => vec![texture.clone(), height.to_string(), ratio.to_string()],
        }
    }
}

/// What preflight learned about one object.
#[derive(Debug, Clone)]
pub struct Preflight {
    /// Resolved `.sco` file.
    pub path: Option<PathBuf>,
    /// Parsed object (`None` when unreadable).
    pub object: Option<SceneryObject>,
    /// Lower-case keywords present in the file.
    pub keywords: BTreeSet<String>,
    /// Context classes.
    pub classes: Vec<String>,
    /// Load-stage failure categories found without rendering.
    pub categories: Vec<String>,
    /// String variables of the placement.
    pub strings: StringPlan,
}

impl Preflight {
    /// `[keyword]` tags of unsupported features.
    #[must_use]
    pub fn unsupported(&self) -> Vec<String> {
        UNSUPPORTED_KEYWORDS
            .iter()
            .filter(|k| self.keywords.contains(**k))
            .map(|k| format!("[{k}]"))
            .collect()
    }

    /// Whether the object has a text texture that the placement's strings feed.
    #[must_use]
    pub fn has_text(&self) -> bool {
        matches!(self.strings, StringPlan::Text(n) if n > 0)
    }
}

/// Parses `entry` and derives tags, classes and strings.
#[must_use]
pub fn run(install: &OmsiInstall, entry: &ObjectEntry) -> Preflight {
    let mut pre = Preflight {
        path: install.find_file(&entry.sco.replace('/', "\\")),
        object: None,
        keywords: BTreeSet::new(),
        classes: Vec::new(),
        categories: Vec::new(),
        strings: StringPlan::None,
    };
    let bytes = pre.path.as_ref().and_then(|p| std::fs::read(p).ok());
    let Some(bytes) = bytes else {
        pre.categories.push("F-SCO-PARSE".into());
        return pre;
    };
    pre.keywords = scan_blocks(&decode_bytes(&bytes))
        .into_iter()
        .map(|b| b.keyword)
        .collect();
    match SceneryObject::parse(&bytes) {
        Ok(o) => pre.object = Some(o),
        Err(e) => {
            tracing::warn!(sco = %entry.sco, "preflight: sco parse failed: {e}");
            pre.categories.push("F-SCO-PARSE".into());
            return pre;
        }
    }
    pre.classes = classes(&pre, entry);
    pre.strings = string_plan(pre.object.as_ref());
    pre
}

mod classify;
mod listing;
use classify::{classes, string_plan};
pub use listing::{survey, SurveyLine};

#[cfg(test)]
mod tests;
