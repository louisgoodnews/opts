//! Turning the world load counters into problems.

use super::*;

/// Counters of the world load that become problems when the loader finishes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorldCounts {
    /// Tiles that were loaded.
    pub tiles_loaded: usize,
    /// Tiles the map lists but whose file is missing.
    pub tiles_missing: usize,
    /// Tiles in the neighbourhood that was asked for.
    pub tiles_total: usize,
    /// Scenery objects that failed (and splines).
    pub objects_failed: usize,
    /// Files that could not be read.
    pub asset_failures: usize,
}

/// Turns the counters into problems (appended to `report`); no tile at all is fatal.
pub fn problems_from_counts(report: &mut LoadReport, map: &str, c: WorldCounts, failures: &str) {
    if c.tiles_loaded == 0 {
        report.add(
            LoadProblem::new(true, "load-problem-map", map, "no map tile could be loaded")
                .because("load-cause-no-tiles"),
        );
        return;
    }
    if c.tiles_missing > 0 {
        report.add(LoadProblem::counted(
            "load-problem-tiles",
            c.tiles_missing,
            c.tiles_total.max(c.tiles_missing),
            format!(
                "{} map tile file(s) listed in global.cfg are missing",
                c.tiles_missing
            ),
        ));
    }
    if c.objects_failed > 0 {
        report.add(LoadProblem::counted(
            "load-problem-objects",
            c.objects_failed,
            0,
            "scenery objects or splines that could not be built",
        ));
    }
    if c.asset_failures > 0 {
        report.add(LoadProblem::counted(
            "load-problem-files",
            c.asset_failures,
            0,
            failures,
        ));
    }
}
