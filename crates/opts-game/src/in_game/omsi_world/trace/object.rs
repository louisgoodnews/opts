//! Recording one placed object: its result, its parents and its trace record.

use super::parents::object_parents;
use super::site::{Counters, Site};
use super::*;

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

/// Records one placed object (scenery, tree, parked vehicle, attachment).
pub(in crate::in_game::omsi_world) fn object(
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
