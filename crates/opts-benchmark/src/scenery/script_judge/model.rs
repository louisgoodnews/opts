//! The judged model: the `[mesh]` rules, per-view masks and the shared judge context.

use super::*;

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

pub(super) fn mesh_infos(sco: &SceneryObject) -> Vec<MeshInfo> {
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

pub(super) fn mesh_masks(
    exp: &Expected,
    place: &Placement,
    views: &[View],
    n: usize,
) -> Vec<Vec<MeshMasks>> {
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
