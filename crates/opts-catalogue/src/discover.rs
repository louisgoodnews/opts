//! The scan that discovers maps and vehicles.

use super::relocation::{best_candidate, normalize};
use super::*;

/// Rebuilds the catalogue for `root` into the database at `db_path`.
///
/// Opens its own connection (safe to run on a background thread while [`Catalogue`] handles
/// serve reads from the previous snapshot). Everything is written in a single transaction.
///
/// # Errors
/// Fails if the root is missing or on SQLite errors.
pub fn discover(db_path: &Path, root: &Path, progress: &(dyn Fn(Phase) + Sync)) -> Result<Summary> {
    if !root.is_dir() {
        return Err(Error::RootMissing(root.to_path_buf()));
    }
    let started = std::time::Instant::now();
    progress(Phase::Walking);
    let scanned = scan::scan_installation(root);
    tracing::info!(
        files = scanned.files.len(),
        dirs = scanned.dirs.len(),
        secs = started.elapsed().as_secs_f32(),
        "filesystem walk finished"
    );

    progress(Phase::Detecting);
    let drafts = extract::detect_entities(root, &scanned.files, &scanned.dirs);
    tracing::info!(
        entities = drafts.len(),
        secs = started.elapsed().as_secs_f32(),
        "entity detection finished"
    );

    progress(Phase::Writing);
    let mut conn = Connection::open(db_path)?;
    schema::prepare(&conn)?;
    let tx = conn.transaction()?;
    tx.execute_batch(
        "DELETE FROM entity_references; DELETE FROM map_info; DELETE FROM vehicle_info;
         DELETE FROM entities; DELETE FROM files; DELETE FROM directories;",
    )?;

    let dir_ids = insert_directories(&tx, &scanned.dirs)?;
    let file_ids = insert_files(&tx, &scanned.files, &dir_ids)?;
    let by_name = build_name_index(&scanned.files);

    let mut summary = Summary {
        files: file_ids.len(),
        directories: dir_ids.len(),
        ..Summary::default()
    };
    let entity_by_marker =
        insert_entities(&tx, &drafts, &file_ids, &dir_ids, &by_name, &mut summary)?;
    link_aliases(&tx, &drafts, &entity_by_marker)?;
    write_metadata(&tx, root, &summary)?;
    tx.commit()?;

    // Fold the write-ahead log back into the main file so the database stays compact.
    let _ = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
    tracing::info!(
        ?summary,
        secs = started.elapsed().as_secs_f32(),
        "catalogue scan committed"
    );
    Ok(summary)
}

fn insert_directories(
    tx: &rusqlite::Transaction<'_>,
    dirs: &[scan::DirRec],
) -> Result<HashMap<String, i64>> {
    let mut dir_ids = HashMap::with_capacity(dirs.len());
    let mut st = tx.prepare(
        "INSERT INTO directories(parent_dir_id, rel_path, dir_name, depth, mtime_sec, child_dir_count, child_file_count)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )?;
    for d in dirs {
        let parent = d.parent.as_ref().and_then(|p| dir_ids.get(p)).copied();
        st.execute(params![
            parent,
            d.rel_path,
            d.name,
            d.depth,
            d.mtime,
            d.child_dirs,
            d.child_files
        ])?;
        dir_ids.insert(d.rel_path.clone(), tx.last_insert_rowid());
    }
    Ok(dir_ids)
}

fn insert_files(
    tx: &rusqlite::Transaction<'_>,
    files: &[scan::FileRec],
    dir_ids: &HashMap<String, i64>,
) -> Result<HashMap<String, i64>> {
    let mut file_ids = HashMap::with_capacity(files.len());
    let mut st = tx.prepare(
        "INSERT INTO files(dir_id, rel_path, file_name, extension, size_bytes, mtime_sec, is_symlink, symlink_target, is_backup)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
    )?;
    for f in files {
        let Some(dir_id) = dir_ids.get(&f.dir) else {
            continue;
        };
        st.execute(params![
            dir_id,
            f.rel_path,
            f.name,
            f.ext,
            f.size,
            f.mtime,
            f.is_symlink,
            f.symlink_target,
            f.is_backup
        ])?;
        file_ids.insert(f.rel_path.to_lowercase(), tx.last_insert_rowid());
    }
    Ok(file_ids)
}

fn build_name_index(files: &[scan::FileRec]) -> HashMap<String, Vec<String>> {
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    for f in files {
        by_name
            .entry(f.name.to_lowercase())
            .or_default()
            .push(f.rel_path.clone());
    }
    by_name
}

fn insert_entity_info(
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

fn resolve_reference(
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

fn insert_entities(
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

fn link_aliases(
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

fn write_metadata(tx: &rusqlite::Transaction<'_>, root: &Path, summary: &Summary) -> Result<()> {
    let root_str = root.to_string_lossy().into_owned();
    for (k, v) in [
        ("omsi_root", root_str),
        ("last_scan_completed_at", now_secs().to_string()),
        ("total_files", summary.files.to_string()),
        ("total_entities", summary.entities.to_string()),
        ("scan_version", SCAN_VERSION.to_string()),
    ] {
        tx.execute(
            "INSERT INTO meta(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![k, v],
        )?;
    }
    Ok(())
}
