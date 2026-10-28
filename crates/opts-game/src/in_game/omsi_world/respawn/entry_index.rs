//! Background worker and resource indexing the map's `[entrypoints]` records to world positions.

use super::*;

/// An `[entrypoints]` record resolved to the position of its placed `entrypoint_*.sco`.
#[derive(Debug, Clone)]
pub struct ResolvedEntry {
    /// Record number of the `[entrypoints]` entry (several records can share a name).
    pub id: u32,
    /// Display name (`global.cfg`).
    pub name: String,
    /// Tile the entry object lies on.
    pub tile: (i32, i32),
    /// Position in that tile's frame, with the record's elevation as `z`.
    pub local: Vec3,
    /// Heading of the entry object (compass degrees).
    pub heading_deg: f32,
}

/// What the worker has found so far, and whether it has finished.
#[derive(Default)]
struct Found {
    entries: Vec<ResolvedEntry>,
    done: bool,
}

type SharedEntries = Arc<Mutex<Found>>;

/// The map's entry points with their positions, built once per session on a worker thread (every
/// tile is read to find the placed entry objects, nearest tiles first). Results are published as
/// they are found, so the entries near the player can be checked long before the last tile is
/// read.
#[derive(Resource, Default)]
pub struct EntryIndex {
    shared: Option<SharedEntries>,
    found: Vec<ResolvedEntry>,
    done: bool,
    map: Option<PathBuf>,
}

impl EntryIndex {
    /// Forgets everything (a new session starts).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Starts resolving the entry points of the loaded map (idempotent).
    pub fn start(&mut self, job: &WorldLoadJob) {
        if self.map.as_ref() == Some(&job.config.map_dir) {
            return;
        }
        self.map = Some(job.config.map_dir.clone());
        self.found.clear();
        self.done = false;
        let start = job.start_tile;
        let mut tiles: Vec<((i32, i32), PathBuf)> = job
            .tile_files
            .keys()
            .filter_map(|k| job.tile_path(*k).map(|p| (*k, p)))
            .collect();
        tiles.sort_by_key(|((x, y), _)| (x - start.0).abs().max((y - start.1).abs()));
        let entries = job.manifest.entry_points.clone();
        let shared: SharedEntries = Arc::new(Mutex::new(Found::default()));
        self.shared = Some(shared.clone());
        std::thread::spawn(move || resolve_entries(&tiles, &entries, &shared));
    }

    /// Takes what the worker has found so far.
    pub fn poll(&mut self) {
        if self.done {
            return;
        }
        if let Some(shared) = &self.shared {
            if let Ok(f) = shared.lock() {
                if f.entries.len() != self.found.len() || f.done != self.done {
                    self.found = f.entries.clone();
                    self.done = f.done;
                }
            }
        }
    }

    /// The entries resolved so far.
    #[must_use]
    pub fn entries(&self) -> &[ResolvedEntry] {
        &self.found
    }

    /// Whether every tile has been read.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// The resolved entry with that record number.
    #[must_use]
    pub fn find(&self, id: u32) -> Option<&ResolvedEntry> {
        self.found.iter().find(|e| e.id == id)
    }
}

/// Reads the tiles and finds the placed object of each entry record (the same rule as
/// `WorldLoadJob::find_entry_spawn`; entries on spline attachments are left out).
fn resolve_entries(
    tiles: &[((i32, i32), PathBuf)],
    entries: &[MapEntryPoint],
    shared: &SharedEntries,
) {
    let mut found = 0usize;
    for (key, path) in tiles {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(tile) = MapTile::parse(&bytes) else {
            continue;
        };
        let mut fresh = Vec::new();
        for ep in entries {
            if let Some(obj) = tile
                .objects
                .iter()
                .find(|o| o.id_code == u64::from(ep.id_code))
            {
                fresh.push(ResolvedEntry {
                    id: ep.id,
                    name: ep.name.clone(),
                    tile: *key,
                    local: Vec3::new(obj.position.x, obj.position.y, ep.position.y),
                    heading_deg: obj.heading_deg,
                });
            }
        }
        if !fresh.is_empty() {
            if let Ok(mut f) = shared.lock() {
                for r in fresh {
                    if !f.entries.iter().any(|e| e.id == r.id) {
                        f.entries.push(r);
                        found += 1;
                    }
                }
            }
        }
        if found >= entries.len() {
            break;
        }
    }
    if let Ok(mut f) = shared.lock() {
        f.done = true;
    }
}
