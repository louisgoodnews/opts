//! Render statistics for the `render.stats` control command (M-0082, mesh instancing research).
//!
//! Three sources, all cheap enough to stay on:
//!
//! * **Frame times** (main world): the real delta of every frame since the last reset, reduced to
//!   average, median, 1 % low (the mean of the slowest 1 % of frames, as a rate) and maximum.
//! * **Render phases** (render world): after queueing, the number of batch sets, bins and entities
//!   of Bevy's binned phases (`Opaque3d`, `AlphaMask3d`, `Shadow`) and the item count of the sorted
//!   `Transparent3d` phase. With GPU preprocessing one *batch set* is one `multi_draw_indirect`
//!   call; a *bin* of a non-multidrawable batch set is one instanced draw; every unbatchable
//!   entity is one draw (`bevy_render::render_phase`, `bevy_core_pipeline::core_3d`).
//! * **Parts** (main world, only when asked): how many `Mesh3d + MeshMaterial3d` entities exist and
//!   are visible, and how many distinct meshes, materials and (mesh, material) pairs they use, which
//!   is the lower bound of the batches Bevy can form.
//!
//! GPU time per pass is available with `OPTS_RENDER_DIAG=1` (Bevy's `RenderDiagnosticsPlugin`,
//! timestamp queries on Vulkan and DX12).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bevy::core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d};
use bevy::diagnostic::DiagnosticsStore;
use bevy::pbr::{MeshMaterial3d, Shadow, StandardMaterial};
use bevy::prelude::*;
use bevy::render::batching::NoAutomaticBatching;
use bevy::render::render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases};
use bevy::render::{Render, RenderApp, RenderSystems};
use serde_json::{json, Value};

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
pub struct SharedRenderPhaseCounts(Arc<Mutex<BTreeMap<&'static str, PhaseCounts>>>);

/// Frame times since the last reset.
#[derive(Resource, Default)]
pub struct FrameTimes {
    /// Seconds per frame.
    pub samples: Vec<f32>,
}

/// Adds the frame-time recorder and the render-world phase counters.
pub struct RenderStatsPlugin;

impl Plugin for RenderStatsPlugin {
    fn build(&self, app: &mut App) {
        let shared = SharedRenderPhaseCounts::default();
        app.insert_resource(shared.clone())
            .init_resource::<FrameTimes>()
            .add_systems(Last, record_frame_time);
        if std::env::var_os("OPTS_RENDER_DIAG").is_some() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        }
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(shared).add_systems(
                Render,
                count_phases.in_set(RenderSystems::Cleanup).before(
                    // After the bins are filled, before the next frame clears them.
                    RenderSystems::PostCleanup,
                ),
            );
        }
    }
}

fn record_frame_time(time: Res<Time<Real>>, mut frames: ResMut<FrameTimes>) {
    if frames.samples.len() < 1_000_000 {
        frames.samples.push(time.delta_secs());
    }
}

fn binned<P: bevy::render::render_phase::BinnedPhaseItem>(
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

fn count_phases(
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

/// Summary of the frame times since the last reset.
#[must_use]
pub fn frame_summary(samples: &[f32]) -> Value {
    if samples.is_empty() {
        return json!({ "frames": 0 });
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f32::total_cmp);
    let n = sorted.len();
    let avg = sorted.iter().sum::<f32>() / n as f32;
    let slow = (n / 100).max(1);
    let low_1pct = sorted[n - slow..].iter().sum::<f32>() / slow as f32;
    let ms = |s: f32| (f64::from(s) * 1000.0 * 1000.0).round() / 1000.0;
    json!({
        "frames": n,
        "avg_ms": ms(avg),
        "median_ms": ms(sorted[n / 2]),
        "p99_ms": ms(sorted[(n * 99 / 100).min(n - 1)]),
        "max_ms": ms(sorted[n - 1]),
        "avg_fps": (1.0 / f64::from(avg) * 10.0).round() / 10.0,
        "low_1pct_fps": (1.0 / f64::from(low_1pct) * 10.0).round() / 10.0,
    })
}

fn memory_kb() -> Value {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        status
            .lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
    };
    json!({ "rss_kb": field("VmRSS:"), "peak_rss_kb": field("VmHWM:") })
}

/// Counts of the mesh parts in the main world.
fn parts(world: &mut World) -> Value {
    let materials = world.resource::<Assets<StandardMaterial>>();
    let alpha: HashMap<AssetId<StandardMaterial>, u8> = materials
        .iter()
        .map(|(id, m)| {
            let kind = match m.alpha_mode {
                AlphaMode::Opaque => 0,
                AlphaMode::Mask(_) => 1,
                _ => 2,
            };
            (id, kind)
        })
        .collect();
    let mut q = world.query::<(
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        Option<&ViewVisibility>,
        Has<NoAutomaticBatching>,
    )>();
    let (mut total, mut visible, mut no_batch) = (0usize, 0usize, 0usize);
    let mut meshes = HashSet::new();
    let mut mats = HashSet::new();
    let mut pairs = HashSet::new();
    let mut per_kind = [0usize; 3];
    let mut mat_users: HashMap<AssetId<StandardMaterial>, usize> = HashMap::new();
    for (mesh, mat, vis, nb) in q.iter(world) {
        total += 1;
        if !vis.is_none_or(|v| v.get()) {
            continue;
        }
        visible += 1;
        no_batch += usize::from(nb);
        meshes.insert(mesh.0.id());
        mats.insert(mat.0.id());
        pairs.insert((mesh.0.id(), mat.0.id()));
        per_kind[usize::from(*alpha.get(&mat.0.id()).unwrap_or(&0))] += 1;
        *mat_users.entry(mat.0.id()).or_default() += 1;
    }
    let single_use = mat_users.values().filter(|n| **n == 1).count();
    json!({
        "mesh_entities": total,
        "visible": visible,
        "visible_distinct_meshes": meshes.len(),
        "visible_distinct_materials": mats.len(),
        "visible_distinct_pairs": pairs.len(),
        "visible_materials_used_once": single_use,
        "visible_opaque_mask_blend": per_kind,
        "no_automatic_batching": no_batch,
    })
}

/// The `render.stats` answer. `reset` clears the frame-time window afterwards.
pub fn render_stats(world: &mut World, reset: bool, with_parts: bool) -> Value {
    let frames = world
        .get_resource::<FrameTimes>()
        .map(|f| frame_summary(&f.samples))
        .unwrap_or(Value::Null);
    let phases: Value = world
        .get_resource::<SharedRenderPhaseCounts>()
        .and_then(|s| s.0.lock().ok().map(|g| g.clone()))
        .map(|m| {
            Value::Object(
                m.into_iter()
                    .map(|(k, c)| {
                        (
                            k.to_owned(),
                            json!({
                                "views": c.views,
                                "multidraw_sets": c.multidraw_sets,
                                "batchable_bins": c.batchable_bins,
                                "unbatchable_entities": c.unbatchable_entities,
                                "sorted_items": c.sorted_items,
                                "api_calls": c.api_calls(),
                            }),
                        )
                    })
                    .collect(),
            )
        })
        .unwrap_or(Value::Null);
    let gpu: Value = world
        .get_resource::<DiagnosticsStore>()
        .map(|store| {
            Value::Object(
                store
                    .iter()
                    .filter(|d| d.path().as_str().starts_with("render/"))
                    .filter_map(|d| {
                        let v = d.average()?;
                        Some((d.path().to_string(), json!((v * 1000.0).round() / 1000.0)))
                    })
                    .collect(),
            )
        })
        .unwrap_or(Value::Null);
    let mut out = json!({
        "frame_times": frames,
        "phases": phases,
        "gpu_diagnostics_ms": gpu,
        "entities": world.entities().len(),
        "memory": memory_kb(),
        "instancing_mode": crate::instancing::mode_name(),
    });
    if with_parts {
        out["parts"] = parts(world);
    }
    if reset {
        if let Some(mut f) = world.get_resource_mut::<FrameTimes>() {
            f.samples.clear();
        }
    }
    out
}
