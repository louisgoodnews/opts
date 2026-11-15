//! Corpus test: the timetable files of every map's `TTData` folder parse and refer to each
//! other (M-0099): the runs of a line file name trip files that exist and profiles they have,
//! the stations of a trip are in the stop list, and the schedule of every run runs forward in
//! time. Skipped unless `OMSI_PATH` points at an installation.

#[path = "timetable_corpus/counts.rs"]
mod counts;
#[path = "timetable_corpus/hidden.rs"]
mod hidden;

use std::path::PathBuf;

/// The `TTData` folder of every map of the installation `OMSI_PATH` names; `None` (skip) without.
fn ttdata_dirs() -> Option<Vec<PathBuf>> {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set: skipped");
        return None;
    };
    let maps = std::fs::read_dir(root.join("maps")).ok()?;
    Some(
        maps.flatten()
            .filter_map(|m| {
                std::fs::read_dir(m.path())
                    .into_iter()
                    .flatten()
                    .flatten()
                    .find(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case("TTData")
                    })
                    .map(|e| e.path())
            })
            .collect(),
    )
}
