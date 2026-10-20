//! What went wrong while loading, kept apart from the status text so it is never overwritten
//! (audit F03): the loading screen ends in *Ready*, *Ready with N problems* or *Failed*
//! (`docs/src/roadmap/phase-9/wizard-and-loading.md` section 7).

use bevy::prelude::*;

/// Start the drive without waiting for *Continue* when the load ends with problems. Set by the
/// control protocol (`loading.continue`, `scene.load_map`) for one load; the command line flag
/// `--accept-load-problems` does the same for every load.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AcceptLoadProblems(pub bool);

/// How much a problem matters for the drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The map, a map tile, or the player's vehicle did not load: the screen waits for the user.
    Blocking,
    /// Individual assets (textures, scenery meshes, fonts, text sources) are missing: the drive
    /// starts by itself and the problems are announced afterwards.
    Minor,
}

/// The one place that says which kind of problem blocks the screen.
pub fn severity_of(key: &str) -> Severity {
    match key {
        "load-problem-objects" | "load-problem-files" => Severity::Minor,
        // load-problem-map, -vehicle, -no-model, -tiles and anything unknown block.
        _ => Severity::Blocking,
    }
}

/// One thing that did not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadProblem {
    /// Nothing usable came out of it (the load failed).
    pub fatal: bool,
    /// Message key of the sentence that says what happened.
    pub key: &'static str,
    /// Variables of that sentence (`name`, `count`, `total`).
    pub args: Vec<(String, String)>,
    /// Message key of the cause in plain words, if one is known.
    pub cause_key: Option<&'static str>,
    /// Paths, error texts and codes for the details view.
    pub detail: String,
}

impl LoadProblem {
    /// How much this problem matters.
    pub fn severity(&self) -> Severity {
        severity_of(self.key)
    }

    /// A problem with one variable.
    pub fn new(fatal: bool, key: &'static str, name: &str, detail: impl Into<String>) -> Self {
        Self {
            fatal,
            key,
            args: vec![("name".to_string(), name.to_string())], // l10n-ok: variable name
            cause_key: None,
            detail: detail.into(),
        }
    }

    /// Adds the plain-language cause.
    #[must_use]
    pub fn because(mut self, cause_key: &'static str) -> Self {
        self.cause_key = Some(cause_key);
        self
    }

    /// A counted problem ("3 of 9 map tiles are missing").
    pub fn counted(
        key: &'static str,
        count: usize,
        total: usize,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            fatal: false,
            key,
            args: vec![
                ("count".to_string(), count.to_string()), // l10n-ok: variable name
                ("total".to_string(), total.to_string()), // l10n-ok: variable name
            ],
            cause_key: None,
            detail: detail.into(),
        }
    }
}

/// How the loading ended (or that it has not yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadOutcome {
    /// Still working.
    #[default]
    Loading,
    /// Done without problems: the screen continues by itself.
    Ready,
    /// Usable, but with problems: the screen stays until the user continues.
    ReadyWithProblems,
    /// Nothing usable: the screen stays and offers retry, another choice and back.
    Failed,
}

/// The problems of the current load and what the user decided about them.
#[derive(Resource, Debug, Clone, Default)]
pub struct LoadReport {
    /// Everything that went wrong, in the order it happened.
    pub problems: Vec<LoadProblem>,
    /// The outcome once the loader has finished.
    pub outcome: LoadOutcome,
    /// The user pressed *Continue* on *Ready with problems*.
    pub continue_requested: bool,
    /// The details view is open.
    pub details_open: bool,
    /// The announcement of minor problems after the first HUD frame was made.
    pub minor_announced: bool,
    /// The user pressed *Retry*; the pipeline starts over at its next step.
    pub retry_requested: bool,
}

impl LoadReport {
    /// Records a problem (never replaced by later status text).
    pub fn add(&mut self, problem: LoadProblem) {
        tracing::warn!(problem = problem.key, detail = %problem.detail, "load problem");
        self.problems.push(problem);
    }

    /// Problems that stop the screen.
    pub fn blocking_count(&self) -> usize {
        self.problems
            .iter()
            .filter(|p| p.severity() == Severity::Blocking)
            .count()
    }

    /// Problems that only get announced after the drive has started.
    pub fn minor_count(&self) -> usize {
        self.count() - self.blocking_count()
    }

    /// Whether a fatal problem was recorded.
    pub fn has_fatal(&self) -> bool {
        self.problems.iter().any(|p| p.fatal)
    }

    /// Number of problems.
    pub fn count(&self) -> usize {
        self.problems.len()
    }

    /// Settles the outcome when the loader has finished: fatal problems fail, any other problem
    /// is *Ready with problems*, none is *Ready*.
    pub fn finish(&mut self) {
        self.outcome = if self.has_fatal() {
            LoadOutcome::Failed
        } else if self.blocking_count() == 0 {
            LoadOutcome::Ready
        } else {
            LoadOutcome::ReadyWithProblems
        };
    }

    /// Forgets everything (retry).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The report as plain text for *Copy* and for bug reports.
    pub fn as_text(&self) -> String {
        let mut out = String::new();
        for (i, p) in self.problems.iter().enumerate() {
            out.push_str(&format!(
                "{}. {} {:?}{}\n   {}\n",
                i + 1,
                p.key,
                p.args,
                if p.fatal { " (fatal)" } else { "" },
                p.detail.replace('\n', "\n   ")
            ));
        }
        out
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outcome_follows_the_problems() {
        let mut r = LoadReport::default();
        r.finish();
        assert_eq!(r.outcome, LoadOutcome::Ready);
        r.add(LoadProblem::new(false, "load-problem-vehicle", "MAN", "x"));
        r.finish();
        assert_eq!(r.outcome, LoadOutcome::ReadyWithProblems);
        r.add(LoadProblem::new(true, "load-problem-map", "Berlin", "y"));
        r.finish();
        assert_eq!(r.outcome, LoadOutcome::Failed);
    }

    #[test]
    fn severity_table_blocking_vs_minor() {
        for (key, want) in [
            ("load-problem-map", Severity::Blocking),
            ("load-problem-vehicle", Severity::Blocking),
            ("load-problem-no-model", Severity::Blocking),
            ("load-problem-tiles", Severity::Blocking),
            ("load-problem-objects", Severity::Minor),
            ("load-problem-files", Severity::Minor),
            ("something-new", Severity::Blocking),
        ] {
            assert_eq!(severity_of(key), want, "{key}");
        }
    }

    #[test]
    fn only_minor_problems_still_end_ready_and_are_kept() {
        let mut r = LoadReport::default();
        r.add(LoadProblem::counted("load-problem-files", 3, 0, "a.tga"));
        r.finish();
        assert_eq!(r.outcome, LoadOutcome::Ready);
        assert_eq!((r.blocking_count(), r.minor_count()), (0, 1));
        r.add(LoadProblem::counted("load-problem-tiles", 1, 4, "t"));
        r.finish();
        assert_eq!(r.outcome, LoadOutcome::ReadyWithProblems);
    }

    #[test]
    fn counters_become_problems_and_no_tile_is_fatal() {
        let mut r = LoadReport::default();
        problems_from_counts(
            &mut r,
            "Mini",
            WorldCounts {
                tiles_loaded: 4,
                tiles_missing: 1,
                tiles_total: 5,
                objects_failed: 2,
                asset_failures: 3,
            },
            "a.sco\nb.o3d",
        );
        assert_eq!(r.count(), 3);
        assert!(!r.has_fatal());
        let mut none = LoadReport::default();
        problems_from_counts(&mut none, "Mini", WorldCounts::default(), "");
        assert!(none.has_fatal());
    }

    #[test]
    fn problems_are_kept_when_more_are_added_and_can_be_copied_as_text() {
        let mut r = LoadReport::default();
        r.add(LoadProblem::new(
            false,
            "load-problem-vehicle",
            "MAN",
            "model missing",
        ));
        r.add(LoadProblem::new(false, "load-problem-tiles", "", "tile 3"));
        assert_eq!(r.count(), 2);
        let t = r.as_text();
        assert!(t.contains("model missing") && t.contains("tile 3"));
        r.reset();
        assert_eq!(r.count(), 0);
    }
}
