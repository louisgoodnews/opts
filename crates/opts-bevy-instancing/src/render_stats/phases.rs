//! The render-world phase counts: the counters of the binned and sorted phases, summed over the views.

use super::*;

/// Counts of one render phase, summed over all views.
#[derive(Debug, Clone, Copy, Default)]
pub struct PhaseCounts {
    /// Views (cameras, shadow cascades) that have this phase.
    pub views: usize,
    /// Multidrawable batch sets: one `multi_draw_indirect` call each.
    pub multidraw_sets: usize,
    /// Batchable, non-multidrawable bins: one instanced draw each.
    pub batchable_bins: usize,
    /// Unbatchable entities: one draw each.
    pub unbatchable_entities: usize,
    /// Items of a sorted phase (one draw each unless the batcher merges neighbours).
    pub sorted_items: usize,
}

impl PhaseCounts {
    /// Draw API calls the phase issues (multidraw calls count once).
    #[must_use]
    pub fn api_calls(&self) -> usize {
        self.multidraw_sets + self.batchable_bins + self.unbatchable_entities + self.sorted_items
    }
}

/// The latest render-world counts, shared with the main world.
#[derive(Resource, Clone, Default)]
pub struct SharedRenderPhaseCounts(pub(super) Arc<Mutex<BTreeMap<&'static str, PhaseCounts>>>);

pub(super) fn binned<P: bevy::render::render_phase::BinnedPhaseItem>(
    phases: Option<&ViewBinnedRenderPhases<P>>,
) -> PhaseCounts {
    let mut c = PhaseCounts::default();
    let Some(phases) = phases else { return c };
    for phase in phases.values() {
        c.views += 1;
        c.multidraw_sets += phase.multidrawable_meshes.len();
        c.batchable_bins += phase.batchable_meshes.len();
        c.unbatchable_entities += phase
            .unbatchable_meshes
            .values()
            .map(|u| u.entities.len())
            .sum::<usize>();
    }
    c
}

pub(super) fn count_phases(
    shared: Res<SharedRenderPhaseCounts>,
    opaque: Option<Res<ViewBinnedRenderPhases<Opaque3d>>>,
    mask: Option<Res<ViewBinnedRenderPhases<AlphaMask3d>>>,
    shadow: Option<Res<ViewBinnedRenderPhases<Shadow>>>,
    transparent: Option<Res<ViewSortedRenderPhases<Transparent3d>>>,
) {
    let mut out = BTreeMap::new();
    out.insert("opaque", binned(opaque.as_deref()));
    out.insert("alpha_mask", binned(mask.as_deref()));
    out.insert("shadow", binned(shadow.as_deref()));
    let mut t = PhaseCounts::default();
    if let Some(phases) = transparent {
        for p in phases.values() {
            t.views += 1;
            t.sorted_items += p.items.len();
        }
    }
    out.insert("transparent", t);
    if let Ok(mut guard) = shared.0.lock() {
        *guard = out;
    }
}
