//! # OPTS Catalogue (`opts-catalogue`)
//!
//! A SQLite cache of an OMSI 2 installation: every directory and file, the discovered content
//! entities (vehicles, maps, scenery objects, …), their display metadata and their references.
//! Menus and loaders query it instead of walking 600,000+ files on every launch.
//!
//! Design: `docs/src/architecture/catalogue-schema.md` (schema, staleness) and
//! `docs/src/architecture/playability-rules.md` (which vehicles are player-selectable).
//!
//! Staleness: the catalogue is stale when it was never completed, was built for a different
//! root, or is older than the caller's maximum age (24 h in the game). A stale catalogue is
//! rebuilt by [`discover`], which writes inside one transaction so readers keep seeing the
//! previous snapshot until the new one commits.

#![warn(missing_docs)]

pub mod extract;
pub mod scan;
pub mod schema;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

/// Catalogue errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// SQLite failure.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    /// The OMSI root does not exist or is not a directory.
    #[error("OMSI root not found: {0}")]
    RootMissing(PathBuf),
    /// The database was written by a newer version of the application.
    #[error("catalogue schema version {found} is newer than supported version {supported}")]
    NewerSchema {
        /// Version found in the database.
        found: i64,
        /// Highest version this build understands.
        supported: i64,
    },
}

/// Result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Version of the scan logic. Bumped when what a scan records changes (new reference kinds,
/// rerouting) so that databases from older builds are rebuilt even if they are recent.
pub const SCAN_VERSION: i64 = 4;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Why (or whether) the catalogue needs a rescan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staleness {
    /// No completed scan exists.
    Missing,
    /// The catalogue was built for a different OMSI root.
    RootChanged,
    /// The last scan is older than the allowed age.
    Expired {
        /// Age of the last scan.
        age_secs: i64,
    },
    /// The catalogue is current.
    Fresh {
        /// Age of the last scan.
        age_secs: i64,
    },
}

impl Staleness {
    /// Whether a rescan is required.
    #[must_use]
    pub fn needs_scan(self) -> bool {
        !matches!(self, Staleness::Fresh { .. })
    }
}

/// Scan phases reported to a progress callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Walking the filesystem.
    Walking,
    /// Detecting entities and reading their metadata.
    Detecting,
    /// Writing to SQLite.
    Writing,
}

/// Counts from a finished scan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// Files recorded.
    pub files: usize,
    /// Directories recorded.
    pub directories: usize,
    /// Entities recorded.
    pub entities: usize,
    /// References recorded.
    pub references: usize,
    /// References whose target does not exist anywhere in the installation.
    pub unresolved: usize,
    /// References whose path was wrong but whose file name exists elsewhere (rerouted).
    pub rerouted: usize,
}

/// A map row for menus.
#[derive(Debug, Clone, PartialEq)]
pub struct MapRecord {
    /// Entity id.
    pub entity_id: i64,
    /// Folder name under `maps/` (stable id).
    pub folder: String,
    /// Display name.
    pub friendly_name: String,
    /// Description.
    pub description: String,
    /// Number of tiles listed in `global.cfg`.
    pub tile_count: i64,
    /// Default depot.
    pub standard_depot: String,
    /// Year range text.
    pub years: String,
    /// Entry point names.
    pub entry_points: Vec<String>,
}

/// A vehicle row for menus.
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleRecord {
    /// Entity id.
    pub entity_id: i64,
    /// `.bus` file stem (stable id).
    pub id: String,
    /// Marker path relative to the OMSI root.
    pub marker_file: String,
    /// Manufacturer.
    pub manufacturer: String,
    /// Model.
    pub model: String,
    /// Default livery.
    pub livery: String,
    /// Description.
    pub description: String,
    /// Curb mass in tonnes.
    pub mass_tonnes: Option<f64>,
}

/// Handle to the catalogue database (thread-safe).
#[derive(Debug)]
pub struct Catalogue {
    path: PathBuf,
    conn: Mutex<Connection>,
}

impl Catalogue {
    /// Opens (creating and migrating if needed) the catalogue at `db_path`.
    ///
    /// # Errors
    /// Fails if the file cannot be opened or was written by a newer schema.
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_path)?;
        schema::prepare(&conn)?;
        Ok(Self {
            path: db_path.to_path_buf(),
            conn: Mutex::new(conn),
        })
    }

    /// Path of the database file.
    #[must_use]
    pub fn db_path(&self) -> &Path {
        &self.path
    }

    fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        // A panic in another thread cannot leave the connection half-written (SQLite is
        // transactional), so a poisoned lock is safe to reuse.
        let guard = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(f(&guard)?)
    }

    fn meta(&self, key: &str) -> Result<Option<String>> {
        self.with(|c| {
            c.query_row("SELECT value FROM meta WHERE key=?1", [key], |r| r.get(0))
                .optional()
        })
    }

    /// OMSI root recorded by the last scan.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn root(&self) -> Result<Option<PathBuf>> {
        Ok(self.meta("omsi_root")?.map(PathBuf::from))
    }

    /// Compares the last completed scan against `root` and `max_age`.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn staleness(&self, root: &Path, max_age: Duration) -> Result<Staleness> {
        let Some(done) = self
            .meta("last_scan_completed_at")?
            .and_then(|v| v.parse::<i64>().ok())
        else {
            return Ok(Staleness::Missing);
        };
        if self
            .meta("scan_version")?
            .and_then(|v| v.parse::<i64>().ok())
            != Some(SCAN_VERSION)
        {
            return Ok(Staleness::Missing);
        }
        if self.meta("omsi_root")?.as_deref() != Some(&root.to_string_lossy()) {
            return Ok(Staleness::RootChanged);
        }
        let age = (now_secs() - done).max(0);
        let limit = i64::try_from(max_age.as_secs()).unwrap_or(i64::MAX);
        Ok(if age > limit {
            Staleness::Expired { age_secs: age }
        } else {
            Staleness::Fresh { age_secs: age }
        })
    }

    /// Resolves an OMSI-relative path (any case, `\` or `/`) to an absolute path, using the
    /// cached index instead of touching the filesystem.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_file(&self, rel: &str) -> Result<Option<PathBuf>> {
        let rel = normalize(rel);
        let Some(root) = self.root()? else {
            return Ok(None);
        };
        let found: Option<String> = self.with(|c| {
            c.prepare_cached("SELECT rel_path FROM files WHERE rel_path = ?1 COLLATE NOCASE")?
                .query_row([&rel], |r| r.get(0))
                .optional()
        })?;
        Ok(found.map(|p| root.join(p)))
    }

    /// All files with the given file name (any case), up to `limit`, as absolute paths.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_all_by_name(&self, name: &str, limit: usize) -> Result<Vec<PathBuf>> {
        let base = normalize(name);
        let base = base.rsplit('/').next().unwrap_or("").to_string();
        let Some(root) = self.root()? else {
            return Ok(Vec::new());
        };
        if base.is_empty() {
            return Ok(Vec::new());
        }
        let found: Vec<String> = self.with(|c| {
            let mut st = c.prepare_cached(
                "SELECT rel_path FROM files WHERE file_name = ?1 COLLATE NOCASE LIMIT ?2",
            )?;
            let rows = st.query_map(params![base, limit as i64], |r| r.get::<_, String>(0))?;
            rows.collect()
        })?;
        Ok(found.into_iter().map(|p| root.join(p)).collect())
    }

    /// Finds a file by name alone (any case), preferring the one closest to `near_rel_dir`.
    ///
    /// Used to repair references whose path is wrong but whose file exists elsewhere. If
    /// `with_alternative_extensions` is set and the exact name is absent, the same stem with the
    /// other image extensions is tried.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_by_name(
        &self,
        name: &str,
        near_rel_dir: &str,
        with_alternative_extensions: bool,
    ) -> Result<Option<PathBuf>> {
        let base = normalize(name);
        let base = base.rsplit('/').next().unwrap_or("").to_string();
        if base.is_empty() {
            return Ok(None);
        }
        let Some(root) = self.root()? else {
            return Ok(None);
        };
        let stem = base
            .rsplit_once('.')
            .map_or(base.as_str(), |(s, _)| s)
            .to_string();
        let mut names = vec![base.clone()];
        if with_alternative_extensions {
            names.extend(
                ["dds", "bmp", "tga", "png", "jpg"]
                    .iter()
                    .map(|e| format!("{stem}.{e}")),
            );
        }
        let near = normalize(near_rel_dir);
        for n in names {
            let found: Vec<String> = self.with(|c| {
                let mut st = c.prepare_cached(
                    "SELECT rel_path FROM files WHERE file_name = ?1 COLLATE NOCASE LIMIT 64",
                )?;
                let rows = st.query_map([&n], |r| r.get(0))?;
                rows.collect()
            })?;
            if let Some(best) = best_candidate(&found, &near) {
                return Ok(Some(root.join(best)));
            }
        }
        Ok(None)
    }

    /// Finds the real location of a referenced file that does not exist at its stated path
    /// (content packs are often installed under other folder names than the map was made with).
    ///
    /// Candidates are the indexed files with the same name. A single candidate is taken; among
    /// several, the one whose folder names match the reference's best (equal trailing folders,
    /// then shared name tokens) wins, but only when it beats the runner-up clearly.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_relocated(&self, referenced_rel: &str) -> Result<Option<PathBuf>> {
        let rel = normalize(referenced_rel);
        let Some((dir, name)) = rel.rsplit_once('/').or(Some(("", rel.as_str()))) else {
            return Ok(None);
        };
        if name.is_empty() {
            return Ok(None);
        }
        let Some(root) = self.root()? else {
            return Ok(None);
        };
        let found: Vec<String> = self.with(|c| {
            let mut st = c.prepare_cached(
                "SELECT rel_path FROM files WHERE file_name = ?1 COLLATE NOCASE LIMIT 256",
            )?;
            let rows = st.query_map([name], |r| r.get(0))?;
            rows.collect()
        })?;
        Ok(pick_relocated(&found, dir).map(|c| root.join(c)))
    }

    /// Resolves an OMSI-relative directory path, like [`Catalogue::find_file`].
    ///
    /// # Errors
    /// SQLite failures.
    pub fn find_dir(&self, rel: &str) -> Result<Option<PathBuf>> {
        let rel = normalize(rel);
        let Some(root) = self.root()? else {
            return Ok(None);
        };
        let found: Option<String> = self.with(|c| {
            c.prepare_cached("SELECT rel_path FROM directories WHERE rel_path = ?1 COLLATE NOCASE")?
                .query_row([&rel], |r| r.get(0))
                .optional()
        })?;
        Ok(found.map(|p| root.join(p)))
    }

    /// All maps, alphabetically by display name.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn maps(&self) -> Result<Vec<MapRecord>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT e.entity_id, d.dir_name, m.friendly_name, m.description, m.tile_count,
                        m.standard_depot, m.years, m.entry_points
                 FROM entities e JOIN map_info m ON m.entity_id = e.entity_id
                 JOIN directories d ON d.dir_id = e.entity_dir_id
                 WHERE e.category = 'map' ORDER BY m.friendly_name COLLATE NOCASE",
            )?;
            let rows = st.query_map([], |r| {
                let eps: String = r.get(7)?;
                Ok(MapRecord {
                    entity_id: r.get(0)?,
                    folder: r.get(1)?,
                    friendly_name: r.get(2)?,
                    description: r.get(3)?,
                    tile_count: r.get(4)?,
                    standard_depot: r.get(5)?,
                    years: r.get(6)?,
                    entry_points: eps.lines().map(str::to_string).collect(),
                })
            })?;
            rows.collect()
        })
    }

    /// Player-selectable vehicles (`is_playable = 1`, primary entities only).
    ///
    /// # Errors
    /// SQLite failures.
    pub fn playable_vehicles(&self) -> Result<Vec<VehicleRecord>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT e.entity_id, f.rel_path, f.file_name, v.manufacturer, v.model, v.livery,
                        v.description, v.mass_tonnes
                 FROM entities e JOIN vehicle_info v ON v.entity_id = e.entity_id
                 JOIN files f ON f.file_id = e.marker_file_id
                 WHERE e.category = 'vehicle' AND e.is_playable = 1 AND e.alias_of_id IS NULL
                 ORDER BY v.manufacturer COLLATE NOCASE, v.model COLLATE NOCASE",
            )?;
            let rows = st.query_map([], |r| {
                let file: String = r.get(2)?;
                Ok(VehicleRecord {
                    entity_id: r.get(0)?,
                    marker_file: r.get(1)?,
                    id: file
                        .rsplit_once('.')
                        .map_or(file.clone(), |(s, _)| s.to_string()),
                    manufacturer: r.get(3)?,
                    model: r.get(4)?,
                    livery: r.get(5)?,
                    description: r.get(6)?,
                    mass_tonnes: r.get(7)?,
                })
            })?;
            rows.collect()
        })
    }

    /// Relative path of the `.bus` marker for a vehicle id (file stem), any case.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn vehicle_marker(&self, id: &str) -> Result<Option<String>> {
        let like = format!("%/{id}.bus");
        self.with(|c| {
            c.query_row(
                "SELECT rel_path FROM files WHERE rel_path LIKE ?1 ESCAPE '\\' COLLATE NOCASE AND extension='bus' LIMIT 1",
                [like.replace('_', "\\_")],
                |r| r.get(0),
            )
            .optional()
        })
    }

    /// Relative paths of all `.hum` human profiles.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn human_markers(&self) -> Result<Vec<String>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT f.rel_path FROM entities e JOIN files f ON f.file_id = e.marker_file_id
                 WHERE e.category = 'human' ORDER BY f.rel_path COLLATE NOCASE",
            )?;
            let rows = st.query_map([], |r| r.get(0))?;
            rows.collect()
        })
    }

    /// Unix time of the last completed scan, if any.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn last_scan(&self) -> Result<Option<i64>> {
        Ok(self
            .meta("last_scan_completed_at")?
            .and_then(|v| v.parse().ok()))
    }

    /// `(files, entities)` counted by the last scan.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn totals(&self) -> Result<(i64, i64)> {
        let get = |k: &str| {
            self.meta(k)
                .map(|v| v.and_then(|v| v.parse().ok()).unwrap_or(0))
        };
        Ok((get("total_files")?, get("total_entities")?))
    }

    /// Counts per entity category (diagnostics).
    ///
    /// # Errors
    /// SQLite failures.
    pub fn category_counts(&self) -> Result<Vec<(String, i64)>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT category, count(*) FROM entities GROUP BY category ORDER BY category",
            )?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect()
        })
    }
}

/// Number of leading path components shared by two `/`-separated paths (case-insensitive).
fn shared_prefix(a: &str, b: &str) -> usize {
    a.split('/')
        .zip(b.split('/'))
        .take_while(|(x, y)| x.eq_ignore_ascii_case(y))
        .count()
}

/// Picks the best of several same-named files for a reference made from `near_dir`:
/// the one sharing the longest leading path, then the shortest path.
fn best_candidate<'a>(candidates: &'a [String], near_dir: &str) -> Option<&'a String> {
    candidates.iter().max_by(|a, b| {
        shared_prefix(a, near_dir)
            .cmp(&shared_prefix(b, near_dir))
            .then_with(|| b.len().cmp(&a.len()))
    })
}

/// Chooses among same-named files for a reference made from `ref_dir`: a single candidate is
/// taken; of several, the best folder match when it beats the runner-up clearly.
fn pick_relocated<'a>(found: &'a [String], ref_dir: &str) -> Option<&'a String> {
    match found {
        [] => None,
        [one] => Some(one),
        _ => {
            let mut scored: Vec<((usize, usize), &String)> = found
                .iter()
                .map(|c| (relocation_score(c, ref_dir), c))
                .collect();
            scored.sort_by_key(|s| std::cmp::Reverse(s.0));
            let (best, second) = (scored[0].0, scored[1].0);
            (best > second && best != (0, 0)).then_some(scored[0].1)
        }
    }
}

/// How well the folders of candidate `cand` match the folders of a reference made from `ref_dir`:
/// `(equal trailing folders, shared name tokens)`.
fn relocation_score(cand: &str, ref_dir: &str) -> (usize, usize) {
    let cand_dir = cand.rsplit_once('/').map_or("", |(d, _)| d);
    let trailing = cand_dir
        .rsplit('/')
        .zip(ref_dir.rsplit('/'))
        .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
        .count();
    const GENERIC: [&str; 6] = [
        "splines",
        "sceneryobjects",
        "vehicles",
        "texture",
        "model",
        "maps",
    ];
    let tokens = |d: &str| -> Vec<String> {
        d.split(|c: char| !c.is_alphanumeric())
            .map(str::to_lowercase)
            .filter(|t| t.len() >= 3 && !GENERIC.contains(&t.as_str()))
            .collect()
    };
    let (a, b) = (tokens(cand_dir), tokens(ref_dir));
    (trailing, a.iter().filter(|t| b.contains(t)).count())
}

fn normalize(rel: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in rel.split(['\\', '/']) {
        match c.trim() {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

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

    let mut dir_ids: HashMap<String, i64> = HashMap::with_capacity(scanned.dirs.len());
    {
        let mut st = tx.prepare(
            "INSERT INTO directories(parent_dir_id, rel_path, dir_name, depth, mtime_sec, child_dir_count, child_file_count)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
        )?;
        for d in &scanned.dirs {
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
    }
    let mut file_ids: HashMap<String, i64> = HashMap::with_capacity(scanned.files.len());
    {
        let mut st = tx.prepare(
            "INSERT INTO files(dir_id, rel_path, file_name, extension, size_bytes, mtime_sec, is_symlink, symlink_target, is_backup)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        )?;
        for f in &scanned.files {
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
    }

    // Same-name index for repairing wrong references (lowercase file name -> relative paths).
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    for f in &scanned.files {
        by_name
            .entry(f.name.to_lowercase())
            .or_default()
            .push(f.rel_path.clone());
    }

    let mut summary = Summary {
        files: file_ids.len(),
        directories: dir_ids.len(),
        ..Summary::default()
    };
    let mut entity_by_marker: HashMap<String, i64> = HashMap::new();
    {
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
        for d in &drafts {
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
            match &d.info {
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
            for (role, raw, candidates) in &d.refs {
                let mut target = candidates
                    .iter()
                    .find_map(|c| file_ids.get(&c.to_lowercase()))
                    .copied();
                let mut state = i64::from(target.is_some());
                if target.is_none() {
                    // Wrong path, right name? Reroute to the closest identically named file.
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
                    if *role == "texture" {
                        names.extend(
                            ["dds", "bmp", "tga", "png", "jpg"]
                                .iter()
                                .map(|e| format!("{stem}.{e}")),
                        );
                    }
                    for n in names {
                        if let Some(paths) = by_name.get(&n) {
                            if let Some(best) = best_candidate(paths, near) {
                                target = file_ids.get(&best.to_lowercase()).copied();
                                if target.is_some() {
                                    state = 2;
                                    break;
                                }
                            }
                        }
                    }
                }
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
    }
    // Alias twins (.ovh next to a same-named .bus).
    {
        let mut upd = tx.prepare("UPDATE entities SET alias_of_id = ?1 WHERE entity_id = ?2")?;
        for d in &drafts {
            if let (Some(primary), Some(me)) = (
                d.alias_of_marker
                    .as_ref()
                    .and_then(|m| entity_by_marker.get(&m.to_lowercase())),
                entity_by_marker.get(&d.marker.to_lowercase()),
            ) {
                upd.execute(params![primary, me])?;
            }
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("opts_cat_{}_{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Vehicles/Bus1/model")).unwrap();
        fs::create_dir_all(dir.join("maps/Town")).unwrap();
        fs::write(
            dir.join("Vehicles/Bus1/Bus1.bus"),
            "[friendlyname]\nACME\nCity 1\nRed\n\n[model]\nmodel\\model.cfg\n\n\
                 [passengercabin]\nmodel\\cabin.cfg\n\n\
                 [add_camera_driver]\n0.5\n0.3\n1.2\n0\n60\n0\n0\n",
        )
        .unwrap();
        fs::write(
            dir.join("Vehicles/Bus1/model/cabin.cfg"),
            "[drivpos]\n0.5\n0.3\n1.0\n0\n",
        )
        .unwrap();
        fs::write(
            dir.join("Vehicles/Bus1/Bus1_trail.bus"),
            "[friendlyname]\nACME\nTrailer\nRed\n",
        )
        .unwrap();
        fs::write(dir.join("Vehicles/Bus1/model/model.cfg"), "x").unwrap();
        fs::write(dir.join("maps/Town/global.cfg"), "[name]\nTown\n").unwrap();
        dir
    }

    #[test]
    fn discovers_entities_and_resolves_paths_case_insensitively() {
        let root = fixture();
        let db = root.join("cat.db");
        let summary = discover(&db, &root, &|_| {}).unwrap();
        assert!(summary.files >= 4 && summary.entities >= 3);

        let cat = Catalogue::open(&db).unwrap();
        let maps = cat.maps().unwrap();
        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].folder, "Town");

        let vehicles = cat.playable_vehicles().unwrap();
        assert_eq!(
            vehicles.len(),
            1,
            "the *_trail.bus segment is not player-selectable"
        );
        assert_eq!(vehicles[0].id, "Bus1");
        assert_eq!(vehicles[0].manufacturer, "ACME");

        let found = cat.find_file("vehicles\\BUS1\\MODEL\\Model.CFG").unwrap();
        assert!(found.is_some_and(|p| p.ends_with("Vehicles/Bus1/model/model.cfg")));
        assert!(cat
            .find_file("Vehicles/Bus1/missing.cfg")
            .unwrap()
            .is_none());
        assert_eq!(
            cat.vehicle_marker("bus1").unwrap().as_deref(),
            Some("Vehicles/Bus1/Bus1.bus")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn staleness_tracks_age_and_root() {
        let root = fixture();
        let db = root.join("cat.db");
        let cat = Catalogue::open(&db).unwrap();
        let day = Duration::from_secs(24 * 3600);
        assert_eq!(cat.staleness(&root, day).unwrap(), Staleness::Missing);
        discover(&db, &root, &|_| {}).unwrap();
        assert!(matches!(
            cat.staleness(&root, day).unwrap(),
            Staleness::Fresh { .. }
        ));
        assert!(matches!(
            cat.staleness(&root, Duration::ZERO).unwrap(),
            Staleness::Fresh { .. } | Staleness::Expired { .. }
        ));
        assert_eq!(
            cat.staleness(Path::new("/elsewhere"), day).unwrap(),
            Staleness::RootChanged
        );
        cat.with(|c| {
            c.execute(
                "UPDATE meta SET value='1' WHERE key='last_scan_completed_at'",
                [],
            )
        })
        .unwrap();
        assert!(matches!(
            cat.staleness(&root, day).unwrap(),
            Staleness::Expired { .. }
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reroutes_references_to_same_named_files_elsewhere() {
        let root = fixture();
        fs::create_dir_all(root.join("Sceneryobjects/PackA/model")).unwrap();
        fs::create_dir_all(root.join("Sceneryobjects/PackB/texture")).unwrap();
        fs::write(
            root.join("Sceneryobjects/PackA/a.sco"),
            "[mesh]\na.o3d\n[matl]\nlost.bmp\n0\n[matl]\ngone.bmp\n1\n",
        )
        .unwrap();
        fs::write(root.join("Sceneryobjects/PackA/model/a.o3d"), b"x").unwrap();
        fs::write(root.join("Sceneryobjects/PackB/texture/LOST.dds"), b"x").unwrap(); // other pack, other extension
        let db = root.join("cat.db");
        let summary = discover(&db, &root, &|_| {}).unwrap();
        assert_eq!(summary.rerouted, 1, "{summary:?}");
        assert_eq!(summary.unresolved, 1, "gone.bmp exists nowhere");
        let cat = Catalogue::open(&db).unwrap();
        let state: i64 = cat
            .with(|c| {
                c.query_row(
                    "SELECT is_resolved FROM entity_references WHERE raw_reference='lost.bmp'",
                    [],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(state, 2);
        let found = cat
            .find_by_name("textures\\lost.bmp", "Sceneryobjects/PackA", true)
            .unwrap();
        assert!(found.is_some_and(|p| p.ends_with("PackB/texture/LOST.dds")));
        assert!(cat.find_by_name("gone.bmp", "", true).unwrap().is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extracts_scenery_and_spline_references() {
        let root = fixture();
        fs::create_dir_all(root.join("Sceneryobjects/Pack/model")).unwrap();
        fs::create_dir_all(root.join("Sceneryobjects/Pack/texture")).unwrap();
        fs::create_dir_all(root.join("Splines/Road/texture")).unwrap();
        fs::create_dir_all(root.join("Texture")).unwrap();
        fs::write(
            root.join("Sceneryobjects/Pack/house.sco"),
            "[friendlyname]\nHouse\n[mesh]\nhouse.o3d\n[matl]\nwall.bmp\n0\n[matl]\nmissing.bmp\n1\n",
        )
        .unwrap();
        fs::write(root.join("Sceneryobjects/Pack/model/house.o3d"), b"x").unwrap();
        fs::write(root.join("Sceneryobjects/Pack/texture/wall.dds"), b"x").unwrap(); // other extension
        fs::write(
            root.join("Splines/Road/street.sli"),
            "[texture]\nasphalt.bmp\n",
        )
        .unwrap();
        fs::write(root.join("Texture/asphalt.bmp"), b"x").unwrap(); // global fallback
        let db = root.join("cat.db");
        discover(&db, &root, &|_| {}).unwrap();
        let cat = Catalogue::open(&db).unwrap();
        let rows: Vec<(String, String, bool)> = cat
            .with(|c| {
                let mut st = c.prepare(
                    "SELECT role, raw_reference, is_resolved FROM entity_references ORDER BY role, raw_reference",
                )?;
                let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? == 1)))?;
                r.collect()
            })
            .unwrap();
        let has = |role: &str, raw: &str, ok: bool| {
            rows.iter().any(|r| r.0 == role && r.1 == raw && r.2 == ok)
        };
        assert!(has("model", "house.o3d", true), "{rows:?}");
        assert!(
            has("texture", "wall.bmp", true),
            "extension fallback: {rows:?}"
        );
        assert!(has("texture", "missing.bmp", false));
        assert!(
            has("texture", "asphalt.bmp", true),
            "global Texture/ fallback"
        );
        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod relocation_tests {
    use super::pick_relocated;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_single_candidate_is_taken() {
        let f = v(&["Splines/Oberstedt_V2/Wilsede_by_Oberpfalz/8m_Standard3.sli"]);
        assert!(pick_relocated(&f, "Splines/Oberpfalz 3D/Wilsede").is_some());
    }

    #[test]
    fn equal_trailing_folders_win() {
        let f = v(&[
            "Sceneryobjects/ADDON_Bad_Huegelsdorf/Odanrebode/Blumenkasten1.sco",
            "Sceneryobjects/St-Servan_Additionnel/Tettau/Blumenkasten/Blumenkasten1.sco",
            "Sceneryobjects/Oberpfalz 3D/Krummenaab2/Blumenkasten1.sco",
        ]);
        let got = pick_relocated(&f, "Sceneryobjects/Tettau/Blumenkasten").unwrap();
        assert!(got.contains("St-Servan_Additionnel/Tettau"));
    }

    #[test]
    fn shared_name_tokens_decide_when_no_folder_matches() {
        let f = v(&[
            "Splines/VKL´s Splines/Zaun.sli",
            "Splines/Altenessen-Splines/Zaun.sli",
        ]);
        let got =
            pick_relocated(&f, "Splines/Oberpfalz 3D/LVB_VKL_Splines_Private_Changes").unwrap();
        assert!(got.contains("VKL"));
    }

    #[test]
    fn ambiguous_candidates_are_not_guessed() {
        let f = v(&["Splines/A/Zaun.sli", "Splines/B/Zaun.sli"]);
        assert!(pick_relocated(&f, "Splines/C").is_none());
    }
}
