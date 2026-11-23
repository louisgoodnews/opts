//! A load job for unit tests of the systems that read it (no map is loaded; the install is the
//! given folder, which may hold only the files a test writes).

use super::*;

/// A job over the install at `root`, with no tiles resident and no splines read yet.
pub(in crate::in_game::omsi_world) fn job_for_tests(root: &Path) -> WorldLoadJob {
    let config = OmsiSessionConfig {
        root: root.to_path_buf(),
        map_dir: root.join("maps").join("Test"),
        vehicle_path: None,
        entry_point: String::new(),
        start_station: None,
        livery: String::new(),
        setvars: Vec::new(),
        human_path: None,
        lookup: None,
        name_lookup: None,
        twin_lookup: None,
        relocate_lookup: None,
    };
    new_job(
        OmsiInstall::new(root),
        config,
        MapManifest::default(),
        HashMap::new(),
    )
}
