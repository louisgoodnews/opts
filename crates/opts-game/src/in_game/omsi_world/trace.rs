//! Scene trace hooks (M-0087): turn what the loaders place into [`TraceRecord`]s.
//!
//! Everything here is behind `opts_bevy::scene_trace::enabled()`; with the trace off the hooks in
//! the loaders reduce to one atomic load. See `docs/src/architecture/scene-trace.md`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
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

static HUMAN_MESHES: Mutex<Vec<(String, Vec<PathBuf>)>> = Mutex::new(Vec::new());

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

fn to_transform(t: &Transform) -> TraceTransform {
    TraceTransform {
        t: t.translation.to_array(),
        q: t.rotation.to_array(),
        s: t.scale.to_array(),
    }
}

fn notes(frame: &Frame) -> Vec<MeshNote> {
    frame.meshes.iter().map(|p| st::note_for(p)).collect()
}

/// Records one placed object (scenery, tree, parked vehicle, attachment).
#[allow(clippy::too_many_arguments)]
pub(super) fn object(
    job: &WorldLoadJob,
    stats: &WorldStats,
    before: Counters,
    frame: Frame,
    obj: &MapObjectInstance,
    site: Site<'_>,
    tile: &MapTile,
    key: (i32, i32),
    origin: (f32, f32),
    entity: Option<Entity>,
    started: Instant,
) {
    if frame.skip {
        return;
    }
    let after = Counters::of(job, stats);
    let mut warnings = frame.warnings.clone();
    let result = if after.loaded > before.loaded {
        "ok"
    } else if after.no_geometry > before.no_geometry || after.hidden > before.hidden {
        "no-geometry"
    } else if after.protected > before.protected {
        warnings.push("no mesh has usable geometry".into());
        "failed"
    } else if after.failed > before.failed {
        "failed"
    } else {
        // Editor-only helpers and markers without a vehicle: nothing was placed.
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

/// Records the terrain of a tile and its splines.
pub(super) fn tile_content(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    key: (i32, i32),
    origin: (f32, f32),
    has_terrain: bool,
    splines_failed_delta: usize,
) {
    let tile_file = job.tile_path(key).map(|p| st::relative(&p));
    let world = omsi_to_bevy_pos(Vec3::new(origin.0, origin.1, 0.0)).to_array();
    let probe = Probe {
        kind: kind::TERRAIN,
        tile: Some(key),
        file: tile_file.as_deref().unwrap_or_default(),
        world,
    };
    st::record(&probe, || TraceRecord {
        kind: kind::TERRAIN.into(),
        tile: Some([key.0, key.1]),
        tile_file: tile_file.clone(),
        file: tile_file.clone().unwrap_or_default(),
        world: Some(TraceTransform {
            t: world,
            q: [0.0, 0.0, 0.0, 1.0],
            s: [1.0; 3],
        }),
        result: if has_terrain { "ok" } else { "no-geometry" }.into(),
        detail: Some(json!({
            "objects": tile.objects.len(),
            "splines": tile.splines.len(),
            "attachments": tile.spline_attachments.len(),
            "version": tile.version,
            "has_terrain": tile.has_terrain,
            "has_water": tile.has_water,
        })),
        ..TraceRecord::default()
    });
    for (i, s) in tile.splines.iter().enumerate() {
        let at = omsi_to_bevy_pos(Vec3::new(
            origin.0 + s.position.x,
            origin.1 + s.position.y,
            s.position.z,
        ));
        let probe = Probe {
            kind: kind::SPLINE,
            tile: Some(key),
            file: &s.sli_file,
            world: at.to_array(),
        };
        let profile = st::wants(&probe).then(|| job.sli(&s.sli_file));
        st::record(&probe, || {
            let mut warnings = Vec::new();
            if profile.as_ref().is_some_and(Option::is_none) {
                warnings.push(format!("profile not found: {}", s.sli_file));
            }
            if splines_failed_delta > 0 {
                warnings.push(format!(
                    "{splines_failed_delta} spline(s) of this tile failed to build"
                ));
            }
            TraceRecord {
                kind: kind::SPLINE.into(),
                tile: Some([key.0, key.1]),
                tile_file: tile_file.clone(),
                index: Some(i),
                list: Some("splines".into()),
                file: s.sli_file.clone(),
                pos: Some(s.position.to_array()),
                rot_deg: Some([s.heading_deg, 0.0, 0.0]),
                world: Some(TraceTransform {
                    t: at.to_array(),
                    q: [0.0, 0.0, 0.0, 1.0],
                    s: [1.0; 3],
                }),
                result: if warnings.is_empty() { "ok" } else { "failed" }.into(),
                warnings,
                detail: Some(json!({
                    "id_code": s.id_code, "length_m": s.length_m, "radius_m": s.radius_m,
                    "gradient": [s.gradient_start, s.gradient_end],
                    "mirrored": s.mirrored, "prev": s.conn_id_1, "next": s.conn_id_2,
                })),
                ..TraceRecord::default()
            }
        });
    }
}

/// Records a vehicle or character that was spawned outside the tiles (traffic car, pedestrian,
/// session vehicle).
#[allow(clippy::too_many_arguments)]
pub(super) fn spawned(
    kind_name: &'static str,
    label: &str,
    file: &str,
    entity: Entity,
    transform: &Transform,
    frame: &Frame,
    result: &str,
    detail: serde_json::Value,
) {
    let meshes = notes(frame);
    let probe = Probe {
        kind: kind_name,
        tile: None,
        file,
        world: transform.translation.to_array(),
    };
    st::record(&probe, || TraceRecord {
        kind: kind_name.into(),
        file: file.to_owned(),
        world: Some(to_transform(transform)),
        bounds: st::union_bounds(&meshes),
        entity: Some(entity.to_bits()),
        meshes,
        result: result.into(),
        warnings: frame.warnings.clone(),
        detail: Some(json!({"name": label, "info": detail})),
        ..TraceRecord::default()
    });
}

/// Remembers the mesh files of a character (they load when the pool fills, not when one walks).
pub(super) fn remember_human(name: &str, frame: &Frame) {
    if !st::enabled() {
        return;
    }
    if let Ok(mut m) = HUMAN_MESHES.lock() {
        m.retain(|(n, _)| n != name);
        m.push((name.to_owned(), frame.meshes.clone()));
    }
}

/// The mesh files remembered for a character.
pub(super) fn human_frame(name: &str) -> Frame {
    let meshes = HUMAN_MESHES
        .lock()
        .ok()
        .and_then(|m| m.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()))
        .unwrap_or_default();
    Frame {
        meshes,
        ..Frame::default()
    }
}
