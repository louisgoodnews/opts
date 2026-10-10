//! Log rotation and historical log retention pruning.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::config::RetentionPolicy;
use crate::error::LogResult;

/// Scans the log directory and prunes historical log files per the configured retention policy.
///
/// Returns the number of files deleted.
pub fn apply_retention(log_dir: &Path, policy: &RetentionPolicy) -> LogResult<usize> {
    if !log_dir.is_dir() {
        return Ok(0);
    }

    match policy {
        RetentionPolicy::Forever => Ok(0),
        RetentionPolicy::Days(days) => prune_older_than(log_dir, *days),
        RetentionPolicy::Count(max_count) => prune_excess_count(log_dir, *max_count),
    }
}

/// Deletes log files whose last modified time is older than the given number of days.
fn prune_older_than(log_dir: &Path, days: u32) -> LogResult<usize> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(u64::from(days) * 86_400))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut deleted = 0;
    let entries = fs::read_dir(log_dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_prunable_log_file(&path) {
            continue;
        }

        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff && fs::remove_file(&path).is_ok() {
                    deleted += 1;
                }
            }
        }
    }

    Ok(deleted)
}

/// Keeps at most `max_count` newest rotated log files per log prefix, deleting the oldest.
fn prune_excess_count(log_dir: &Path, max_count: u32) -> LogResult<usize> {
    let mut files_by_prefix: BTreeMap<String, Vec<(SystemTime, std::path::PathBuf)>> =
        BTreeMap::new();

    let entries = fs::read_dir(log_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_prunable_log_file(&path) {
            continue;
        }

        let prefix = extract_prefix(&path);
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        files_by_prefix
            .entry(prefix)
            .or_default()
            .push((modified, path));
    }

    let mut deleted = 0;
    for files in files_by_prefix.values_mut() {
        // Sort newest first
        files.sort_by_key(|b| std::cmp::Reverse(b.0));

        if files.len() > max_count as usize {
            for (_, old_path) in &files[max_count as usize..] {
                if fs::remove_file(old_path).is_ok() {
                    deleted += 1;
                }
            }
        }
    }

    Ok(deleted)
}

/// Identifies whether a file is a candidate for retention pruning.
/// We do not prune `panics.log` unless explicitly configured.
fn is_prunable_log_file(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    // Keep panics.log safe from routine rotation pruning
    if file_name == "panics.log" {
        return false;
    }

    file_name.ends_with(".log") || file_name.contains(".log.")
}

/// Extracts a common prefix for grouping rolled files (e.g. "opts.log.2026-10-08" -> "opts").
fn extract_prefix(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    if let Some(pos) = name.find(".log") {
        name[..pos].to_string()
    } else {
        name.to_string()
    }
}
