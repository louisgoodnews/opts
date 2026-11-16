//! Entity metadata and reference resolution during the scan.

use super::*;

pub(super) fn insert_entity_info(
    map_info: &mut rusqlite::Statement<'_>,
    veh_info: &mut rusqlite::Statement<'_>,
    id: i64,
    info: &extract::Info,
) -> Result<()> {
    match info {
        extract::Info::Map {
            friendly_name,
            description,
            tile_count,
            standard_depot,
            years,
            entry_points,
        } => {
            map_info.execute(params![
                id,
                friendly_name,
                description,
                tile_count,
                standard_depot,
                years,
                entry_points
            ])?;
        }
        extract::Info::Vehicle {
            manufacturer,
            model,
            livery,
            description,
            mass_tonnes,
        } => {
            veh_info.execute(params![
                id,
                manufacturer,
                model,
                livery,
                description,
                mass_tonnes
            ])?;
        }
        extract::Info::None => {}
    }
    Ok(())
}

pub(super) fn resolve_reference(
    raw: &str,
    role: &str,
    candidates: &[String],
    file_ids: &HashMap<String, i64>,
    by_name: &HashMap<String, Vec<String>>,
) -> (Option<i64>, i64) {
    let target = candidates
        .iter()
        .find_map(|c| file_ids.get(&c.to_lowercase()))
        .copied();
    if target.is_some() {
        return (target, 1);
    }
    // Wrong path, right name? Reroute to closest identically named file.
    let name = normalize(raw);
    let name = name.rsplit('/').next().unwrap_or("").to_lowercase();
    let stem = name
        .rsplit_once('.')
        .map_or(name.as_str(), |(s, _)| s)
        .to_string();
    let near = candidates
        .first()
        .map_or("", |c| c.rsplit_once('/').map_or("", |(d, _)| d));
    let mut names = vec![name.clone()];
    if role == "texture" {
        names.extend(
            ["dds", "bmp", "tga", "png", "jpg"]
                .iter()
                .map(|e| format!("{stem}.{e}")),
        );
    }
    for n in names {
        if let Some(paths) = by_name.get(&n) {
            if let Some(best) = best_candidate(paths, near) {
                if let Some(found) = file_ids.get(&best.to_lowercase()).copied() {
                    return (Some(found), 2);
                }
            }
        }
    }
    (None, 0)
}
