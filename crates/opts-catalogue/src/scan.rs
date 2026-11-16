//! Parallel filesystem walk producing directory and file records.

use std::path::Path;

use rayon::prelude::*;

mod records;
mod walk;

pub use records::{DirRec, FileRec, ScanOut};
use walk::walk;

#[cfg(test)]
mod tests;

/// Scans the whole installation. The first two directory levels are walked sequentially,
/// everything below runs in parallel (one rayon task per depth-2 directory).
#[must_use]
pub fn scan_installation(root: &Path) -> ScanOut {
    let mut out = ScanOut::default();
    let mut level1 = Vec::new();
    walk(root, "", None, 0, false, &mut out, &mut level1);

    let mut level2 = Vec::new();
    for (path, rel, depth) in level1 {
        let parent = rel.rsplit_once('/').map_or("", |(p, _)| p).to_string();
        walk(
            &path,
            &rel,
            Some(&parent),
            depth,
            false,
            &mut out,
            &mut level2,
        );
    }

    let parts: Vec<ScanOut> = level2
        .par_iter()
        .map(|(path, rel, depth)| {
            let mut o = ScanOut::default();
            let parent = rel.rsplit_once('/').map_or("", |(p, _)| p).to_string();
            let mut none = Vec::new();
            walk(path, rel, Some(&parent), *depth, true, &mut o, &mut none);
            o
        })
        .collect();
    for p in parts {
        out.merge(p);
    }
    out.sort();
    out
}
