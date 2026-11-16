//! The scan that discovers maps and vehicles.

use super::relocation::{best_candidate, normalize};
use super::*;

mod insert;
mod entities;
mod links;

use insert::*;
use entities::*;
use links::*;

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
