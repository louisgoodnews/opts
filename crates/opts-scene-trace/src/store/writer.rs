//! The JSONL writer: opening the file of a session, writing lines and closing it.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::json;

use super::*;

impl Store {
    pub fn open_file_if_ready(&mut self) {
        if self.writer.is_some() || self.map.is_none() {
            return;
        }
        let dir = directory();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let map = self.map.clone().unwrap_or_default();
        let safe: String = map
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let path = dir.join(format!("scene-trace-{stamp}-{safe}.jsonl"));
        let Ok(file) = File::create(&path) else {
            tracing::warn!(path = %path.display(), "scene trace file could not be created");
            return;
        };
        self.writer = Some(BufWriter::new(file));
        self.file = Some(path.clone());
        self.bytes = 0;
        self.truncated = false;
        let header = json!({
            "type": "header",
            "schema": 1,
            "build": super::super::build_info(),
            "map": map,
            "mode": self.mode.name(),
            "started_unix": stamp,
            "filters": {
                "tiles": self.filters.tiles,
                "kinds": self.filters.kinds,
                "file": self.filters.file,
            },
            "size_cap": self.size_cap,
        });
        self.write_value(&header);
        tracing::info!(path = %path.display(), mode = self.mode.name(), "scene trace file opened");
        prune(&dir);
    }

    pub fn close(&mut self, why: &str) {
        if self.writer.is_some() {
            let footer = json!({
                "type": "footer", "why": why, "records": self.seen,
                "issues": self.issues, "truncated": self.truncated,
            });
            self.write_value_raw(&footer);
            if let Some(w) = self.writer.as_mut() {
                let _ = w.flush();
            }
            tracing::info!(
                records = self.seen,
                issues = self.issues,
                file = ?self.file,
                "scene trace closed: {why}"
            );
        }
        self.writer = None;
    }

    pub(super) fn write_value(&mut self, v: &impl Serialize) {
        if self.truncated {
            return;
        }
        if self.bytes >= self.size_cap {
            self.truncated = true;
            self.write_value_raw(&json!({"type": "truncated", "cap": self.size_cap}));
            tracing::warn!(cap = self.size_cap, "scene trace file reached its size cap");
            return;
        }
        self.write_value_raw(v);
    }

    pub(super) fn write_value_raw(&mut self, v: &impl Serialize) {
        let Some(w) = self.writer.as_mut() else {
            return;
        };
        if let Ok(line) = serde_json::to_string(v) {
            if writeln!(w, "{line}").is_ok() {
                self.bytes += line.len() as u64 + 1;
            }
        }
    }
}
