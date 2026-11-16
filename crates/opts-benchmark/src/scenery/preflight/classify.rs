//! Context classes and string plans of a preflighted object: what its placement needs.

use opts_core::formats::scenery::SceneryObject;

use super::{ObjectEntry, Preflight, StringPlan};

pub(super) fn classes(pre: &Preflight, entry: &ObjectEntry) -> Vec<String> {
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

pub(super) fn string_plan(o: Option<&SceneryObject>) -> StringPlan {
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
