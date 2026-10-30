//! Scene trace hooks (M-0087): turn what the loaders place into [`TraceRecord`]s.
//!
//! Everything here is behind `opts_bevy::scene_trace::enabled()`; with the trace off the hooks in
//! the loaders reduce to one atomic load. See `docs/src/architecture/scene-trace.md`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::scene_trace::{
    self as st, kind, MeshNote, ParentRef, Probe, TraceRecord, TraceTransform,
};
use opts_core::formats::map::{MapObjectInstance, MapTile};
use serde_json::json;

use super::{WorldLoadJob, WorldStats};

/// What the loaders learn while one thing is loaded (collected in a stack, because a parked-car
/// marker loads the car it stands for from inside its own load).
#[derive(Default)]
pub(super) struct Frame {
    /// Mesh files the thing used.
    pub meshes: Vec<PathBuf>,
    /// Problems seen.
    pub warnings: Vec<String>,
    /// The thing is not a placed thing of its own (a parked-car marker): no record.
    pub skip: bool,
    /// Overrides the record kind (`tree`, `parked`).
    pub kind: Option<&'static str>,
    /// World transform the loader computed.
    pub transform: Option<Transform>,
    /// `.cfg` of a vehicle-style model.
    pub model_cfg: Option<String>,
}

thread_local! {
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
}

mod records;
pub(super) use records::{human_frame, remember_human, spawned, tile_content};

/// Opens a frame (no-op while the trace is off).
pub(super) fn begin_frame() {
    if st::enabled() {
        FRAMES.with(|f| f.borrow_mut().push(Frame::default()));
    }
}

/// Closes the innermost frame.
pub(super) fn end_frame() -> Frame {
    if !st::enabled() {
        return Frame::default();
    }
    FRAMES.with(|f| f.borrow_mut().pop()).unwrap_or_default()
}

fn with_frame(f: impl FnOnce(&mut Frame)) {
    if st::enabled() {
        FRAMES.with(|fr| {
            if let Some(top) = fr.borrow_mut().last_mut() {
                f(top);
            }
        });
    }
}

/// A mesh file was loaded for the thing in progress.
pub(super) fn mesh_used(path: &Path) {
    with_frame(|f| {
        if !f.meshes.iter().any(|p| p == path) {
            f.meshes.push(path.to_path_buf());
        }
    });
}

/// A problem with the thing in progress.
pub(super) fn warn(text: impl FnOnce() -> String) {
    with_frame(|f| f.warnings.push(text()));
}

/// The thing in progress is not a placed thing of its own.
pub(super) fn skip_record() {
    with_frame(|f| f.skip = true);
}

/// Sets the kind of the thing in progress.
pub(super) fn set_kind(k: &'static str) {
    with_frame(|f| f.kind = Some(k));
}

/// The world transform of the thing in progress.
pub(super) fn set_transform(t: &Transform) {
    with_frame(|f| f.transform = Some(*t));
}

/// The vehicle-style model of the thing in progress.
pub(super) fn set_model_cfg(cfg: &str) {
    with_frame(|f| f.model_cfg = Some(cfg.to_owned()));
}

/// Where in the map a placement came from.
#[derive(Clone, Copy)]
pub(super) struct Site<'a> {
    /// `objects` or `attachments`.
    pub list: &'static str,
    /// Index in that list.
    pub index: usize,
    /// For a parked car: the marker object it stands for.
    pub marker: Option<&'a str>,
}

/// Counters before a placement, compared with the counters after it.
#[derive(Clone, Copy, Default)]
pub(super) struct Counters {
    loaded: usize,
    failed: usize,
    no_geometry: usize,
    protected: usize,
    hidden: usize,
}

impl Counters {
    /// Reads the counters.
    pub fn of(job: &WorldLoadJob, stats: &WorldStats) -> Self {
        Self {
            loaded: stats.objects_loaded,
            failed: stats.objects_failed,
            no_geometry: stats.objects_no_geometry,
            protected: stats.objects_protected,
            hidden: job.cache.stats.hidden_parts,
        }
    }
}

pub(super) fn to_transform(t: &Transform) -> TraceTransform {
    TraceTransform {
        t: t.translation.to_array(),
        q: t.rotation.to_array(),
        s: t.scale.to_array(),
    }
}

pub(super) fn notes(frame: &Frame) -> Vec<MeshNote> {
    frame.meshes.iter().map(|p| st::note_for(p)).collect()
}

/// What the object's placement came to, from the counters before and after: `None` for nothing
/// placed (editor-only helpers and markers without a vehicle).
fn placement_result(
    before: Counters,
    after: Counters,
    warnings: &mut Vec<String>,
) -> Option<&'static str> {
    if after.loaded > before.loaded {
        Some("ok")
    } else if after.no_geometry > before.no_geometry || after.hidden > before.hidden {
        Some("no-geometry")
    } else if after.protected > before.protected {
        warnings.push("no mesh has usable geometry".into());
        Some("failed")
    } else if after.failed > before.failed {
        Some("failed")
    } else {
        // Editor-only helpers and markers without a vehicle: nothing was placed.
        None
    }
}

/// The parents of an object: the marker it belongs to, the object an attached object hangs on and
/// the spline an attachment follows.
fn object_parents(
    site: &Site<'_>,
    tile: &MapTile,
    obj: &MapObjectInstance,
    key: (i32, i32),
) -> Vec<ParentRef> {
    let mut parents = Vec::new();
    if let Some(marker) = site.marker {
        parents.push(ParentRef {
            kind: "marker".into(),
            file: marker.to_owned(),
            tile: Some([key.0, key.1]),
            index: Some(site.index),
            id: None,
        });
    }
    if site.list == "attached objects" {
        if let Some(att) = tile.attached_objects.get(site.index) {
            let parent = att.parent_id;
            let file = tile
                .objects
                .iter()
                .find(|o| o.id_code == parent)
                .map(|o| o.sco_file.clone())
                .or_else(|| {
                    tile.attached_objects
                        .iter()
                        .find(|o| o.id_code == parent)
                        .map(|o| o.sco_file.clone())
                })
                .or_else(|| {
                    tile.spline_attachments
                        .iter()
                        .find(|o| o.id_code == parent)
                        .map(|o| o.sco_file.clone())
                })
                .unwrap_or_default();
            parents.push(ParentRef {
                kind: "attach_parent".into(),
                file,
                tile: Some([key.0, key.1]),
                index: None,
                id: None,
            });
        }
    }
    if site.list == "attachments" {
        if let Some(att) = tile
            .spline_attachments
            .iter()
            .find(|a| a.id_code == obj.id_code)
        {
            parents.push(ParentRef {
                kind: kind::SPLINE.into(),
                file: tile
                    .splines
                    .get(att.spline_index)
                    .map(|s| s.sli_file.clone())
                    .unwrap_or_default(),
                tile: Some([key.0, key.1]),
                index: Some(att.spline_index),
                id: None,
            });
        }
    }
    parents
}

/// Records one placed object (scenery, tree, parked vehicle, attachment).
pub(super) fn object(
    job: &WorldLoadJob,
    stats: &WorldStats,
    (before, started): (Counters, Instant),
    frame: Frame,
    (obj, site): (&MapObjectInstance, Site<'_>),
    (tile, key, origin): (&MapTile, (i32, i32), (f32, f32)),
    entity: Option<Entity>,
) {
    if frame.skip {
        return;
    }
    let after = Counters::of(job, stats);
    let mut warnings = frame.warnings.clone();
    let Some(result) = placement_result(before, after, &mut warnings) else {
        return;
    };
    let transform = frame.transform.unwrap_or_else(|| {
        Transform::from_translation(omsi_to_bevy_pos(Vec3::new(
            origin.0 + obj.position.x,
            origin.1 + obj.position.y,
            obj.position.z,
        )))
    });
    let mut kind_name = frame.kind.unwrap_or(if site.list == "attachments" {
        kind::ATTACHMENT
    } else {
        kind::SCENERY
    });
    if frame.model_cfg.is_some() {
        kind_name = kind::PARKED;
    }
    let parents = object_parents(&site, tile, obj, key);
    let meshes = notes(&frame);
    let probe = Probe {
        kind: kind_name,
        tile: Some(key),
        file: &obj.sco_file,
        world: transform.translation.to_array(),
    };
    let tile_file = job.tile_path(key).map(|p| st::relative(&p));
    st::record(&probe, || TraceRecord {
        kind: kind_name.into(),
        tile: Some([key.0, key.1]),
        tile_file,
        index: Some(site.index),
        list: Some(site.list.into()),
        file: obj.sco_file.clone(),
        pos: Some(obj.position.to_array()),
        rot_deg: Some([
            obj.heading_deg,
            obj.pitch_deg.unwrap_or(0.0),
            obj.roll_deg.unwrap_or(0.0),
        ]),
        world: Some(to_transform(&transform)),
        bounds: st::union_bounds(&meshes),
        entity: entity.map(Entity::to_bits),
        meshes,
        parents,
        result: result.into(),
        warnings,
        load_ms: started.elapsed().as_secs_f32() * 1000.0,
        detail: Some(json!({
            "id_code": obj.id_code,
            "model_cfg": frame.model_cfg,
            "string_vars": obj.string_vars,
        })),
        ..TraceRecord::default()
    });
}
