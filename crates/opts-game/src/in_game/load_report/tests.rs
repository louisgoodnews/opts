//! Unit tests for `super` (`load_report.rs`).

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
