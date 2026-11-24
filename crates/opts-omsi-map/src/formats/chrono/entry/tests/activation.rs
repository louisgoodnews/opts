//! Activation and application order of the entries of a `ChronoSet`.

use super::super::*;

/// Active ⇔ start ≤ date ≤ end, with the end inclusive and missing bounds open.
#[test]
fn activation_is_inclusive_and_open_ended() {
    let d = ChronoDate::from_ymd;
    let e = ChronoEntry {
        start: Some(d(1989, 11, 10)),
        end: Some(d(1990, 6, 30)),
        ..ChronoEntry::default()
    };
    assert!(!e.is_active(d(1989, 11, 9)));
    assert!(e.is_active(d(1989, 11, 10)));
    assert!(e.is_active(d(1990, 6, 30)));
    assert!(!e.is_active(d(1990, 7, 1)));
    let open = ChronoEntry::default();
    assert!(open.is_active(d(1, 1, 1)) && open.is_active(d(9999, 12, 31)));
}

/// Order: start date (missing first), then folder path case-insensitively.
#[test]
fn entries_are_applied_by_start_date_then_folder() {
    let d = ChronoDate::from_ymd;
    let mk = |rel: &str, start: Option<ChronoDate>| ChronoEntry {
        rel: rel.into(),
        start,
        ..ChronoEntry::default()
    };
    let set = ChronoSet::from_entries(
        Path::new("/x"),
        vec![
            mk("b", Some(d(1990, 1, 1))),
            mk("Z_undated", None),
            mk("A", Some(d(1990, 1, 1))),
            mk("c", Some(d(1980, 1, 1))),
        ],
    );
    let order: Vec<&str> = set.entries().iter().map(|e| e.rel.as_str()).collect();
    assert_eq!(order, vec!["Z_undated", "c", "A", "b"]);
    assert_eq!(set.active(d(1985, 1, 1)).indices(), &[0, 1]);
}
