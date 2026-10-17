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

fn classes(pre: &Preflight, entry: &ObjectEntry) -> Vec<String> {
    let Some(o) = &pre.object else {
        return Vec::new();
    };
    let has = |k: &str| pre.keywords.contains(k);
    let mut c = Vec::new();
    let mut push = |cond: bool, name: &str| {
        if cond {
            c.push(name.to_string());
        }
    };
    push(o.is_editor_only(), "editor-only");
    push(o.is_help_arrow, "help-arrow");
    push(o.carpark_p, "carpark-marker");
    push(o.tree.is_some(), "tree");
    push(o.model_cfg.is_some(), "vehicle-model");
    push(
        o.meshes.is_empty() && o.tree.is_none() && o.model_cfg.is_none() && !o.carpark_p,
        "no-geometry",
    );
    push(
        o.surface || has("splinehelper") || has("crossing_heightdeformation"),
        "surface",
    );
    push(entry.tier == "attachment", "spline-attachment");
    if c.is_empty() {
        c.push("standalone".into());
    }
    c
}

fn string_plan(o: Option<&SceneryObject>) -> StringPlan {
    let Some(o) = o else {
        return StringPlan::None;
    };
    if let Some(t) = &o.tree {
        return StringPlan::Tree {
            texture: t.texture.clone(),
            height: (t.min_height + t.max_height) * 0.5,
            ratio: (t.min_width_ratio + t.max_width_ratio) * 0.5,
        };
    }
    let n = o
        .text_textures
        .iter()
        .filter(|t| t.string_var.is_none())
        .map(|t| t.string_index + 1)
        .max()
        .unwrap_or(0);
    if n == 0 {
        StringPlan::None
    } else {
        StringPlan::Text(n)
    }
}

/// One line of `scenery preflight`: what the CPU-only preflight knows about an object.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SurveyLine {
    /// Relative `.sco` path.
    pub sco: String,
    /// Tier (`referenced`, `attachment`, `unreferenced`).
    pub tier: String,
    /// Context classes.
    pub class: Vec<String>,
    /// Unsupported-feature tags.
    pub unsupported: Vec<String>,
    /// Preflight failure categories.
    pub categories: Vec<String>,
    /// Number of text-texture strings the placement carries.
    pub text_strings: usize,
}

/// Runs the preflight over `entries` (no rendering) and returns one line per object.
#[must_use]
pub fn survey(install: &OmsiInstall, entries: &[ObjectEntry]) -> Vec<SurveyLine> {
    entries
        .iter()
        .map(|e| {
            let p = run(install, e);
            SurveyLine {
                sco: e.sco.clone(),
                tier: e.tier.clone(),
                unsupported: p.unsupported(),
                text_strings: match p.strings {
                    StringPlan::Text(n) => n,
                    _ => 0,
                },
                class: p.classes,
                categories: p.categories,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_sets_differ_and_are_never_empty() {
        let plan = StringPlan::Text(3);
        let (a, b) = (plan.strings('A'), plan.strings('B'));
        assert_eq!(a, vec!["OPTS-A1", "OPTS-A2", "OPTS-A3"]);
        assert_eq!(b.len(), 3);
        assert!(a.iter().zip(&b).all(|(x, y)| x != y && !x.is_empty()));
    }

    #[test]
    fn tree_plan_writes_texture_height_ratio() {
        let plan = StringPlan::Tree {
            texture: "tree.tga".into(),
            height: 12.5,
            ratio: 0.75,
        };
        assert_eq!(plan.strings('A'), vec!["tree.tga", "12.5", "0.75"]);
    }
}
