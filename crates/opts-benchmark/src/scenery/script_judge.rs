//! Judgement of the script-effect check: turns the captured states into `S-*` categories
//! (`docs/src/architecture/scenery-benchmark.md`, Script-effect check): errors, consistency of the
//! rendered silhouette with the `[visible]` rules, determinism, and (in [`super::script_stimuli`])
//! liveness and lights.

use opts_core::formats::scenery::SceneryObject;

use super::expected::{Expected, Placement};
use super::framing::{View, IMAGE_SIZE};
use super::metrics::{self, ExpectedMasks, Matte};
use super::raster::Mask;
use super::script_check::{ScriptCheck, ScriptThresholds};
use super::script_run::{Outcome, State};
use super::script_stimuli;

mod consistency;
use consistency::{consistency, determinism};

mod checks;
mod model;

use checks::{bound_vars, changed_vars, errors};
use model::{mesh_infos, mesh_masks};
pub use model::{var, Judge, MeshInfo, MeshMasks};

/// Most mismatch strings kept in the record.
const MAX_MISMATCHES: usize = 12;
/// Most changed variables listed in the record.
const MAX_CHANGED_VARS: usize = 40;

/// Result of the judgement.
#[derive(Debug, Clone, Default)]
pub struct Judgement {
    /// The record block.
    pub check: ScriptCheck,
    /// `S-*` categories that make the object fail.
    pub categories: Vec<String>,
    /// The scripts' bound variables never changed (verdict `unknown`, not a failure).
    pub inert: bool,
    /// Mattes of every captured state and view, named `<state>_v1`, `<state>_v2`.
    pub shots: Vec<(String, Matte)>,
}

/// Judges the outcome of the scenarios.
#[must_use]
pub fn judge(
    outcome: &Outcome,
    sco: &SceneryObject,
    exp: &Expected,
    place: &Placement,
    views: &[View],
    th: &ScriptThresholds,
) -> Judgement {
    let meshes = mesh_infos(sco);
    let masks = mesh_masks(exp, place, views, meshes.len());
    let mattes: Vec<Vec<Matte>> = outcome
        .states
        .iter()
        .map(|s| {
            s.pixels
                .iter()
                .map(|p| metrics::matte(&p.black, &p.white))
                .collect()
        })
        .collect();
    let j = Judge {
        outcome,
        meshes,
        masks,
        mattes,
        sco,
        views,
        th,
    };
    let mut out = Judgement::default();
    out.check.states = outcome.states.iter().map(|s| s.name.clone()).collect();
    errors(&j, &mut out);
    changed_vars(&j, &mut out.check);
    bound_vars(&j, &mut out.check);
    if !j.masks.is_empty() {
        consistency(&j, &mut out);
    }
    determinism(&j, &mut out);
    script_stimuli::judge(&j, &mut out);
    for (s, m) in outcome.states.iter().zip(&j.mattes) {
        for (v, matte) in m.iter().enumerate() {
            out.shots
                .push((format!("{}_v{}", s.name, v + 1), matte.clone()));
        }
    }
    out.check.status = out
        .categories
        .first()
        .cloned()
        .unwrap_or_else(|| out.check.status.clone());
    out
}

pub(super) fn add(out: &mut Judgement, category: &str) {
    if !out.categories.iter().any(|c| c == category) {
        out.categories.push(category.to_string());
    }
}

#[cfg(test)]
mod tests;
