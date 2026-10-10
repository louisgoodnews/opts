//! SQLite schema (`docs/src/architecture/catalogue-schema.md`, Stage 7) and migrations.

use rusqlite::Connection;

use crate::Result;

/// Schema version written to `meta.schema_version`.
pub const SCHEMA_VERSION: i64 = 1;

/// Version-1 DDL: the documented tables, indexes and views, plus the two display-info tables
/// the menus need so that no config file has to be parsed at runtime.
const DDL_V1: &str = r"
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS directories (
    dir_id           INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_dir_id    INTEGER REFERENCES directories(dir_id) ON DELETE CASCADE,
    rel_path         TEXT NOT NULL UNIQUE,
    dir_name         TEXT NOT NULL,
    depth            INTEGER NOT NULL,
    mtime_sec        INTEGER NOT NULL,
    child_dir_count  INTEGER NOT NULL DEFAULT 0,
    child_file_count INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS files (
    file_id        INTEGER PRIMARY KEY AUTOINCREMENT,
    dir_id         INTEGER NOT NULL REFERENCES directories(dir_id) ON DELETE CASCADE,
    rel_path       TEXT NOT NULL UNIQUE,
    file_name      TEXT NOT NULL,
    extension      TEXT NOT NULL,
    size_bytes     INTEGER NOT NULL,
    mtime_sec      INTEGER NOT NULL,
    is_symlink     INTEGER NOT NULL DEFAULT 0,
    symlink_target TEXT,
    is_backup      INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS entities (
    entity_id        INTEGER PRIMARY KEY AUTOINCREMENT,
    category         TEXT NOT NULL,
    entity_mode      TEXT NOT NULL CHECK(entity_mode IN ('folder', 'file')),
    name             TEXT NOT NULL,
    marker_file_id   INTEGER NOT NULL REFERENCES files(file_id) ON DELETE CASCADE,
    entity_dir_id    INTEGER NOT NULL REFERENCES directories(dir_id) ON DELETE CASCADE,
    is_playable      INTEGER NOT NULL DEFAULT 1,
    vehicle_class    TEXT CHECK(vehicle_class IN ('road', 'rail', 'aircraft')),
    alias_of_id      INTEGER REFERENCES entities(entity_id) ON DELETE SET NULL,
    created_at_mtime INTEGER NOT NULL
);
-- is_resolved: 0 = missing, 1 = exact path exists, 2 = rerouted to a same-named file elsewhere
CREATE TABLE IF NOT EXISTS entity_references (
    reference_id     INTEGER PRIMARY KEY AUTOINCREMENT,
    source_entity_id INTEGER NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE,
    target_file_id   INTEGER REFERENCES files(file_id) ON DELETE SET NULL,
    raw_reference    TEXT NOT NULL,
    role             TEXT NOT NULL,
    is_resolved      INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS map_info (
    entity_id      INTEGER PRIMARY KEY REFERENCES entities(entity_id) ON DELETE CASCADE,
    friendly_name  TEXT NOT NULL,
    description    TEXT NOT NULL,
    tile_count     INTEGER NOT NULL,
    standard_depot TEXT NOT NULL,
    years          TEXT NOT NULL,
    entry_points   TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS vehicle_info (
    entity_id    INTEGER PRIMARY KEY REFERENCES entities(entity_id) ON DELETE CASCADE,
    manufacturer TEXT NOT NULL,
    model        TEXT NOT NULL,
    livery       TEXT NOT NULL,
    description  TEXT NOT NULL,
    mass_tonnes  REAL
);

CREATE INDEX IF NOT EXISTS idx_directories_parent ON directories(parent_dir_id);
CREATE INDEX IF NOT EXISTS idx_directories_depth_mtime ON directories(depth, mtime_sec);
CREATE INDEX IF NOT EXISTS idx_directories_rel_path_nocase ON directories(rel_path COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_files_dir ON files(dir_id);
CREATE INDEX IF NOT EXISTS idx_files_rel_path_nocase ON files(rel_path COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_files_name_nocase ON files(file_name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_files_ext_dir ON files(extension, dir_id);
CREATE INDEX IF NOT EXISTS idx_files_symlink ON files(is_symlink) WHERE is_symlink = 1;
CREATE INDEX IF NOT EXISTS idx_entities_category_name ON entities(category, name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_entities_playable ON entities(category, is_playable, name COLLATE NOCASE) WHERE is_playable = 1;
CREATE INDEX IF NOT EXISTS idx_entities_vehicle_class ON entities(vehicle_class) WHERE vehicle_class IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_entities_dir ON entities(entity_dir_id);
CREATE INDEX IF NOT EXISTS idx_entities_marker ON entities(marker_file_id);
CREATE INDEX IF NOT EXISTS idx_ref_source ON entity_references(source_entity_id, role);
CREATE INDEX IF NOT EXISTS idx_ref_target ON entity_references(target_file_id, role);
CREATE INDEX IF NOT EXISTS idx_ref_unresolved ON entity_references(is_resolved) WHERE is_resolved = 0;

CREATE VIEW IF NOT EXISTS v_drivable_vehicles AS
SELECT e.entity_id, e.name, e.vehicle_class, f.rel_path AS marker_file, d.rel_path AS vehicle_folder,
       f.size_bytes AS marker_size, e.created_at_mtime
FROM entities e JOIN files f ON e.marker_file_id = f.file_id JOIN directories d ON e.entity_dir_id = d.dir_id
WHERE e.category = 'vehicle' AND e.is_playable = 1;
CREATE VIEW IF NOT EXISTS v_ai_vehicles AS
SELECT e.entity_id, e.name, e.vehicle_class, f.rel_path AS marker_file, d.rel_path AS vehicle_folder
FROM entities e JOIN files f ON e.marker_file_id = f.file_id JOIN directories d ON e.entity_dir_id = d.dir_id
WHERE e.category = 'vehicle' AND e.is_playable = 0;
CREATE VIEW IF NOT EXISTS v_maps AS
SELECT e.entity_id, e.name, d.rel_path AS map_folder, f.rel_path AS global_cfg_path,
       d.child_dir_count AS tile_count, e.created_at_mtime
FROM entities e JOIN files f ON e.marker_file_id = f.file_id JOIN directories d ON e.entity_dir_id = d.dir_id
WHERE e.category = 'map';
CREATE VIEW IF NOT EXISTS v_missing_references AS
SELECT r.reference_id, e.category AS source_category, e.name AS source_entity_name,
       d.rel_path AS source_folder, r.role, r.raw_reference
FROM entity_references r JOIN entities e ON r.source_entity_id = e.entity_id
JOIN directories d ON e.entity_dir_id = d.dir_id
WHERE r.is_resolved = 0;
";

/// Opens-time setup: pragmas, schema creation and the version handshake.
///
/// # Errors
/// Fails on SQLite errors or when the database was written by a newer schema version.
pub fn prepare(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;",
    )?;
    let has_meta: bool = conn
        .query_row(
            "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='meta'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if has_meta {
        let version: Option<i64> = conn
            .query_row(
                "SELECT value FROM meta WHERE key='schema_version'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|v| v.parse().ok());
        if let Some(v) = version {
            if v > SCHEMA_VERSION {
                return Err(crate::Error::NewerSchema {
                    found: v,
                    supported: SCHEMA_VERSION,
                });
            }
        }
    }
    conn.execute_batch(DDL_V1)?;
    conn.execute(
        "INSERT INTO meta(key,value) VALUES('schema_version', ?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [SCHEMA_VERSION.to_string()],
    )?;
    Ok(())
}
