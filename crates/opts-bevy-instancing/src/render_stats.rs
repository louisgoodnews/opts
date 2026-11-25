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

use bevy::camera::RenderTarget;
use bevy::core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d};
use bevy::diagnostic::DiagnosticsStore;
use bevy::light::NotShadowCaster;
use bevy::pbr::{MeshMaterial3d, Shadow, StandardMaterial};
use bevy::prelude::*;
use bevy::render::batching::NoAutomaticBatching;
use bevy::render::render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases};
use bevy::render::{Render, RenderApp, RenderSystems};
use serde_json::{json, Value};

mod frames;
mod phases;
mod scene;

pub use frames::*;
pub use phases::*;
use scene::*;

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
        "instancing_mode": crate::mode_name(),
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
