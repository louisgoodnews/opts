//! Entities and alias links of the scan.

use super::*;

pub(super) fn insert_entities(
    tx: &rusqlite::Transaction<'_>,
    drafts: &[extract::EntityDraft],
    file_ids: &HashMap<String, i64>,
    dir_ids: &HashMap<String, i64>,
    by_name: &HashMap<String, Vec<String>>,
    summary: &mut Summary,
) -> Result<HashMap<String, i64>> {
    let mut entity_by_marker = HashMap::new();
    let mut ins = tx.prepare(
        "INSERT INTO entities(category, entity_mode, name, marker_file_id, entity_dir_id, is_playable, vehicle_class, created_at_mtime)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
    )?;
    let mut map_info = tx.prepare(
        "INSERT INTO map_info(entity_id, friendly_name, description, tile_count, standard_depot, years, entry_points)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )?;
    let mut veh_info = tx.prepare(
        "INSERT INTO vehicle_info(entity_id, manufacturer, model, livery, description, mass_tonnes) VALUES (?1,?2,?3,?4,?5,?6)",
    )?;
    let mut ref_ins = tx.prepare(
        "INSERT INTO entity_references(source_entity_id, target_file_id, raw_reference, role, is_resolved) VALUES (?1,?2,?3,?4,?5)",
    )?;

    for d in drafts {
        let (Some(marker_id), Some(dir_id)) =
            (file_ids.get(&d.marker.to_lowercase()), dir_ids.get(&d.dir))
        else {
            continue;
        };
        ins.execute(params![
            d.category,
            d.mode,
            d.name,
            marker_id,
            dir_id,
            d.is_playable,
            d.vehicle_class,
            d.mtime
        ])?;
        let id = tx.last_insert_rowid();
        entity_by_marker.insert(d.marker.to_lowercase(), id);
        insert_entity_info(&mut map_info, &mut veh_info, id, &d.info)?;

        for (role, raw, candidates) in &d.refs {
            let (target, state) = resolve_reference(raw, role, candidates, file_ids, by_name);
            ref_ins.execute(params![id, target, raw, role, state])?;
            summary.references += 1;
            match state {
                0 => summary.unresolved += 1,
                2 => summary.rerouted += 1,
                _ => {}
            }
        }
        summary.entities += 1;
    }
    Ok(entity_by_marker)
}

pub(super) fn link_aliases(
    tx: &rusqlite::Transaction<'_>,
    drafts: &[extract::EntityDraft],
    entity_by_marker: &HashMap<String, i64>,
) -> Result<()> {
    let mut upd = tx.prepare("UPDATE entities SET alias_of_id = ?1 WHERE entity_id = ?2")?;
    for d in drafts {
        if let (Some(primary), Some(me)) = (
            d.alias_of_marker
                .as_ref()
                .and_then(|m| entity_by_marker.get(&m.to_lowercase())),
            entity_by_marker.get(&d.marker.to_lowercase()),
        ) {
            upd.execute(params![primary, me])?;
        }
    }
    Ok(())
}
