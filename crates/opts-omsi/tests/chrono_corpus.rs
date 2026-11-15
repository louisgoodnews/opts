//! Corpus test of Chrono (`docs/src/formats/map/chrono.md`): activation reproduces the masks
//! cached in `timeline.prt.cfg`, overlay selections resolve when the entries are applied in
//! order, and an element created by one entry is applied before the entries that edit it.
//! Skipped unless `OMSI_PATH` points at an installation.

#[path = "chrono_corpus/berlin.rs"]
mod berlin;
#[path = "chrono_corpus/creators.rs"]
mod creators;
#[path = "chrono_corpus/masks.rs"]
mod masks;
#[path = "chrono_corpus/overlay.rs"]
mod overlay;

use std::path::{Path, PathBuf};

fn maps_with_chrono(root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(root.join("maps"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}
