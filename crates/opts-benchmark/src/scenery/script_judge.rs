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

/// Rules of one `[mesh]` of the `.sco`.
#[derive(Debug, Clone)]
pub struct MeshInfo {
    /// `[visible] <variable> <value>`.
    pub visible: Option<(String, f32)>,
    /// `(variable, factor)` of each `[newanim]` step.
    pub anim: Vec<(String, f32)>,
    /// In the highest-detail LOD group (drawn by `SceneryLodMode::Finest`).
    pub top: bool,
}

/// Per-mesh masks of one view.
pub struct MeshMasks {
    /// Every drawable triangle of the mesh.
    pub all: Mask,
    /// Triangles drawn without alpha test.
    pub solid: Mask,
}

/// Everything the checks share.
pub struct Judge<'a> {
    /// The scenario outcome.
    pub outcome: &'a Outcome,
    /// Rules per `[mesh]`.
    pub meshes: Vec<MeshInfo>,
    /// Masks `[view][mesh]`; empty without expected geometry.
    pub masks: Vec<Vec<MeshMasks>>,
    /// Mattes `[state][view]`.
    pub mattes: Vec<Vec<Matte>>,
    /// The object's lights (variable, factor, size).
    pub sco: &'a SceneryObject,
    /// The views.
    pub views: &'a [View],
    /// Thresholds.
    pub th: &'a ScriptThresholds,
}

/// Value of a script variable in a state (0 for names the engine never saw).
#[must_use]
pub fn var(state: &State, name: &str) -> f32 {
    state
        .probe
        .vars
        .get(&name.to_lowercase())
        .copied()
        .unwrap_or(0.0)
}

impl Judge<'_> {
    /// Whether mesh `i` is shown by its `[visible]` rule in `state`.
    #[must_use]
    pub fn visible(&self, i: usize, state: &State) -> bool {
        self.meshes[i]
            .visible
            .as_ref()
            .is_none_or(|(name, want)| (var(state, name) - want).abs() < 1e-3)
    }

    /// Whether any mesh is animated.
    #[must_use]
    pub fn has_anim(&self) -> bool {
        self.meshes.iter().any(|m| !m.anim.is_empty())
    }

    /// Union of the `all` (or `solid`) masks of the meshes `pick` selects, in view `v`.
    pub fn union(&self, v: usize, solid: bool, pick: impl Fn(usize) -> bool) -> Mask {
        let size = IMAGE_SIZE as usize;
        let mut m = Mask::new(size, size);
        for (i, mm) in self.masks[v].iter().enumerate() {
            if pick(i) {
                m.union_with(if solid { &mm.solid } else { &mm.all });
            }
        }
        m
    }
}

fn mesh_infos(sco: &SceneryObject) -> Vec<MeshInfo> {
    let top = sco
        .meshes
        .iter()
        .filter_map(|m| m.lod_threshold)
        .fold(None, |a: Option<f32>, b| Some(a.map_or(b, |a| a.max(b))));
    sco.meshes
        .iter()
        .map(|m| MeshInfo {
            visible: m.visible.clone(),
            anim: m
                .animations
                .iter()
                .flat_map(|a| a.anim_rot.iter().chain(a.anim_trans.iter()))
                .map(|a| (a.variable.clone(), a.factor))
                .collect(),
            top: m.lod_threshold.is_none_or(|l| Some(l) == top),
        })
        .collect()
}

fn mesh_masks(exp: &Expected, place: &Placement, views: &[View], n: usize) -> Vec<Vec<MeshMasks>> {
    if exp.tris.is_empty() {
        return Vec::new();
    }
    views
        .iter()
        .map(|view| {
            (0..n)
                .map(|i| {
                    let tris: Vec<_> = exp.tris.iter().filter(|t| t.mesh == i).copied().collect();
                    let ExpectedMasks { all, solid, .. } =
                        metrics::expected_masks(&tris, place, view);
                    MeshMasks { all, solid }
                })
                .collect()
        })
        .collect()
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

/// Check 5: diagnostics.
fn errors(j: &Judge<'_>, out: &mut Judgement) {
    if !j.outcome.engine_found {
        add(out, "S-NOT-RUN");
        out.check
            .errors
            .push("the object spawned without a script engine".into());
    }
    for d in &j.outcome.diagnostics {
        let missing = d.message == "file not found";
        let text = format!(
            "{}: {}",
            d.file.rsplit(['/', '\\']).next().unwrap_or(""),
            d.message
        );
        if missing {
            add(out, "S-MISSING");
        } else if d.error || d.runtime {
            add(out, "S-ERROR");
        } else {
            continue;
        }
        if out.check.errors.len() < 8 {
            out.check.errors.push(text);
        }
    }
}

/// The bound variables and their values per state.
fn bound_vars(j: &Judge<'_>, check: &mut ScriptCheck) {
    let mut names: Vec<String> = Vec::new();
    for m in &j.meshes {
        names.extend(m.visible.iter().map(|v| v.0.to_lowercase()));
        names.extend(m.anim.iter().map(|a| a.0.to_lowercase()));
    }
    names.extend(
        j.sco
            .lights
            .iter()
            .filter(|l| !l.variable.trim().is_empty())
            .map(|l| l.variable.trim().to_lowercase()),
    );
    names.sort();
    names.dedup();
    for n in names.into_iter().take(12) {
        let vals = j.outcome.states.iter().map(|s| var(s, &n)).collect();
        check.bound.insert(n, vals);
    }
}

fn changed_vars(j: &Judge<'_>, check: &mut ScriptCheck) {
    let mut names: Vec<&String> = j
        .outcome
        .states
        .iter()
        .flat_map(|s| s.probe.vars.keys())
        .collect();
    names.sort();
    names.dedup();
    for n in names {
        let vals: Vec<f32> = j
            .outcome
            .states
            .iter()
            .map(|s| s.probe.vars.get(n).copied().unwrap_or(0.0))
            .collect();
        if vals.iter().any(|v| (v - vals[0]).abs() > 1e-4)
            && check.changed_vars.len() < MAX_CHANGED_VARS
        {
            check.changed_vars.push(n.clone());
        }
    }
}

#[cfg(test)]
mod tests;
