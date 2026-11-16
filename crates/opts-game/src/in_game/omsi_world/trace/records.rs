//! Trace record emitters for tile terrain/splines, spawned entities, and character meshes.

use std::sync::Mutex;

use super::*;

mod tile;
pub(in super::super) use tile::tile_content;

static HUMAN_MESHES: Mutex<Vec<(String, Vec<PathBuf>)>> = Mutex::new(Vec::new());

/// Records a vehicle or character that was spawned outside the tiles (traffic car, pedestrian,
/// session vehicle).
pub(in super::super) fn spawned(
    kind_name: &'static str,
    label: &str,
    file: &str,
    (entity, transform): (Entity, &Transform),
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
pub(in super::super) fn remember_human(name: &str, frame: &Frame) {
    if !st::enabled() {
        return;
    }
    if let Ok(mut m) = HUMAN_MESHES.lock() {
        m.retain(|(n, _)| n != name);
        m.push((name.to_owned(), frame.meshes.clone()));
    }
}

/// The mesh files remembered for a character.
pub(in super::super) fn human_frame(name: &str) -> Frame {
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
