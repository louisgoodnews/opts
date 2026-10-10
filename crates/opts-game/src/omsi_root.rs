//! Locating the user's OMSI 2 installation.

use std::path::{Path, PathBuf};

/// Well-known Steam install locations, tried after `--omsi-path` and `$OMSI_PATH`.
const CANDIDATES: [&str; 2] = [
    "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2",
    "/home/louisgoodnews/.steam/steam/steamapps/common/OMSI 2",
];

/// Returns the OMSI installation root: `explicit` (CLI), then `$OMSI_PATH`, then known locations.
///
/// A directory only counts as an installation if it contains a `maps` folder.
#[must_use]
pub fn locate_omsi_root(explicit: Option<&Path>) -> Option<PathBuf> {
    let env = std::env::var_os("OMSI_PATH").map(PathBuf::from);
    explicit
        .map(Path::to_path_buf)
        .into_iter()
        .chain(env)
        .chain(CANDIDATES.iter().map(PathBuf::from))
        .find(|p| p.join("maps").is_dir())
}
