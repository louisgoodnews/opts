//! Query and summary helpers for the scene trace store.

use super::*;

impl Store {
    pub fn status(&self) -> Status {
        Status {
            mode: self.mode.name().into(),
            file: self.file.as_ref().map(|p| p.display().to_string()),
            bytes: self.bytes,
            truncated: self.truncated,
            records: self.seen,
            in_memory: self.records.len(),
            issues: self.issues,
            map: self.map.clone(),
            tiles: self.tiles.len(),
        }
    }

    pub fn summary_of(&self, key: (i32, i32)) -> Option<TileSummary> {
        self.tiles.get(&key).cloned()
    }

    pub fn summaries(&self) -> Vec<TileSummary> {
        self.tiles.values().cloned().collect()
    }

    pub fn query(&self, q: &Query) -> Vec<TraceRecord> {
        let limit = if q.limit == 0 { 200 } else { q.limit };
        let mut out = Vec::new();
        for r in &self.records {
            if q.tile.is_some_and(|t| r.tile != Some([t.0, t.1])) {
                continue;
            }
            if q.kind.as_deref().is_some_and(|k| r.kind != k) {
                continue;
            }
            if q.issues_only && !r.is_issue() {
                continue;
            }
            if let Some(p) = &q.file {
                let hit = super::super::filter::glob_match(p, &r.file)
                    || r.meshes
                        .iter()
                        .any(|m| super::super::filter::glob_match(p, &m.file));
                if !hit {
                    continue;
                }
            }
            if let Some(st) = &q.stage {
                if !r.meshes.iter().any(|m| &m.stage == st) {
                    continue;
                }
            }
            if let Some((c, rad)) = q.near {
                let Some(w) = r.world else { continue };
                let (dx, dz) = (w.t[0] - c[0], w.t[2] - c[1]);
                if dx * dx + dz * dz > rad * rad {
                    continue;
                }
            }
            out.push(r.clone());
            if out.len() >= limit {
                break;
            }
        }
        out
    }
}

pub(super) fn prune(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(SystemTime, PathBuf)> = rd
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("scene-trace-"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    files.sort();
    while files.len() > KEEP_FILES {
        let (_, p) = files.remove(0);
        let _ = std::fs::remove_file(p);
    }
}
