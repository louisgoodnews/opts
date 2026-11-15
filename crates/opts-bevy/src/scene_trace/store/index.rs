//! The in-memory index: the records of a tile and the tile lifecycle (begin, end, forget).

use super::*;

impl Store {
    pub fn begin_tile(&mut self, key: (i32, i32)) {
        let had = self.tiles.remove(&key).is_some();
        self.records.retain(|r| r.tile != Some([key.0, key.1]));
        if had {
            self.write_value(&json!({"type": "tile_reload", "tile": [key.0, key.1]}));
        }
    }

    pub fn end_tile(&mut self, key: (i32, i32)) {
        if let Some(s) = self.tiles.get(&key).cloned() {
            self.write_value(&json!({"type": "tile_summary", "summary": s}));
        }
    }

    pub fn forget_tile(&mut self, key: (i32, i32)) {
        self.records.retain(|r| r.tile != Some([key.0, key.1]));
        self.write_value(&json!({"type": "tile_unload", "tile": [key.0, key.1]}));
    }

    pub fn add(&mut self, rec: &mut TraceRecord) -> u64 {
        rec.id = self.next_id;
        self.next_id += 1;
        rec.t_ms = self.started.elapsed().as_millis() as u64;
        self.seen += 1;
        let issue = rec.is_issue();
        if rec.result.is_empty() {
            rec.result = if issue { "failed" } else { "ok" }.into();
        }
        if issue {
            self.issues += 1;
        }
        if let Some(t) = rec.tile {
            let s = self
                .tiles
                .entry((t[0], t[1]))
                .or_insert_with(|| TileSummary {
                    tile: t,
                    ..TileSummary::default()
                });
            s.total += 1;
            *s.kinds.entry(rec.kind.clone()).or_default() += 1;
            if issue {
                s.issue_total += 1;
                *s.issues.entry(rec.kind.clone()).or_default() += 1;
            }
            let mut stages: Vec<&str> = rec.meshes.iter().map(|m| m.stage.as_str()).collect();
            stages.sort_unstable();
            stages.dedup();
            for st in stages {
                *s.stages.entry(st.to_owned()).or_default() += 1;
            }
        }
        let keep = self.mode == Mode::Full || issue;
        if keep {
            let line = json!({"type": "record", "record": &*rec});
            self.write_value(&line);
            if self.records.len() < MAX_MEMORY_RECORDS {
                self.records.push(rec.clone());
            }
        }
        rec.id
    }
}
