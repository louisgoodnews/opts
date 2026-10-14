//! `capture.prune`: deletes old captures this subsystem wrote. Opt-in; a dry run unless told
//! otherwise; nothing is ever deleted automatically.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use bevy::ecs::world::World;
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};

use crate::control::params::{opt_bool, u64_param};
use crate::control::state::{ControlRegistry, Outcome};

use super::error::CaptureError;
use super::storage::CaptureState;

/// Registers `capture.prune`.
pub fn register(r: &mut ControlRegistry) {
    r.register("capture.prune", handle);
}

/// A capture item: a file with its sidecar, or a directory holding a `manifest.json`.
#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    /// The file or directory.
    pub path: PathBuf,
    /// Its sidecar (files only).
    pub sidecar: Option<PathBuf>,
    /// Total size in bytes.
    pub bytes: u64,
}

fn dir_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map_or(0, |rd| {
        rd.flatten()
            .map(|e| {
                let p = e.path();
                if p.is_dir() {
                    dir_size(&p)
                } else {
                    e.metadata().map_or(0, |m| m.len())
                }
            })
            .sum()
    })
}

/// Finds the capture items below `root` (top level and one level of directories) whose newest
/// modification is older than `cutoff`.
#[must_use]
pub fn find_old(root: &Path, cutoff: SystemTime) -> Vec<Item> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else {
        return out;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".json") {
                continue;
            }
            let side = PathBuf::from(format!("{}.json", path.display()));
            if side.is_file() && meta.modified().is_ok_and(|m| m < cutoff) {
                let bytes = meta.len() + std::fs::metadata(&side).map_or(0, |m| m.len());
                out.push(Item {
                    path,
                    sidecar: Some(side),
                    bytes,
                });
            }
        } else if meta.is_dir() && path.join("manifest.json").is_file() {
            let newest = std::fs::metadata(path.join("manifest.json"))
                .and_then(|m| m.modified())
                .ok();
            if newest.is_some_and(|m| m < cutoff) {
                out.push(Item {
                    bytes: dir_size(&path),
                    path,
                    sidecar: None,
                });
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn handle(world: &mut World, params: &Value) -> Outcome {
    match run(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn run(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let days = u64_param(params, "older_than_days")?;
    let dry_run = opt_bool(params, "dry_run")?.unwrap_or(true);
    let root = world
        .get_resource_or_insert_with(CaptureState::default)
        .root
        .clone()
        .ok_or_else(|| CaptureError::Failed("there is no captures directory".into()))?;
    if !root.is_dir() {
        return Ok(json!({ "dry_run": dry_run, "items": [], "bytes": 0 }));
    }
    let root = root
        .canonicalize()
        .map_err(|e| CaptureError::io("resolve", &root, &e))?;
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(days.saturating_mul(86_400)))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let items = find_old(&root, cutoff);
    let mut listed = Vec::new();
    let mut total = 0;
    for item in &items {
        if !dry_run {
            let real = item
                .path
                .canonicalize()
                .map_err(|e| CaptureError::io("resolve", &item.path, &e))?;
            if !real.starts_with(&root) {
                continue;
            }
            let res = if real.is_dir() {
                std::fs::remove_dir_all(&real)
            } else {
                std::fs::remove_file(&real)
            };
            res.map_err(|e| CaptureError::io("delete", &real, &e))?;
            if let Some(side) = &item.sidecar {
                let _ = std::fs::remove_file(side);
            }
        }
        total += item.bytes;
        listed.push(json!({ "path": item.path.display().to_string(), "bytes": item.bytes }));
    }
    Ok(json!({ "dry_run": dry_run, "items": listed, "bytes": total }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_captures_with_a_sidecar_or_manifest_are_candidates() {
        let root = std::env::temp_dir().join(format!("opts_prune_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("burst")).expect("dir");
        std::fs::write(root.join("a.png"), b"x").expect("file");
        std::fs::write(root.join("a.png.json"), b"{}").expect("sidecar");
        std::fs::write(root.join("mine.txt"), b"not a capture").expect("other");
        std::fs::write(root.join("burst").join("manifest.json"), b"{}").expect("manifest");
        std::fs::create_dir_all(root.join("other")).expect("dir");
        std::fs::write(root.join("other").join("keep.png"), b"x").expect("keep");
        let far = SystemTime::now() + Duration::from_secs(3600);
        let found: Vec<String> = find_old(&root, far)
            .iter()
            .map(|i| {
                i.path
                    .file_name()
                    .expect("name")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(found, ["a.png", "burst"]);
        assert!(
            find_old(&root, SystemTime::UNIX_EPOCH).is_empty(),
            "nothing is old yet"
        );
    }
}
