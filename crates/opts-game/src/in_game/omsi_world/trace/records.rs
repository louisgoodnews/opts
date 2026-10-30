//! Trace record emitters for tile terrain/splines, spawned entities, and character meshes.

use std::sync::Mutex;

use super::*;

static HUMAN_MESHES: Mutex<Vec<(String, Vec<PathBuf>)>> = Mutex::new(Vec::new());

/// The record of spline `i` of a tile: its transform, its profile and whether it built.
fn record_spline(
    job: &mut WorldLoadJob,
    (i, s): (usize, &opts_core::formats::map::MapSplineInstance),
    (key, origin): ((i32, i32), (f32, f32)),
    (tile_file, splines_failed_delta): (&Option<String>, usize),
) {
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

/// Records the terrain of a tile and its splines.
pub(in super::super) fn tile_content(
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
        record_spline(
            job,
            (i, s),
            (key, origin),
            (&tile_file, splines_failed_delta),
        );
    }
}

/// Records a vehicle or character that was spawned outside the tiles (traffic car, pedestrian,
/// session vehicle).
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn spawned(
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
