//! Unit tests for `super` (`list_model.rs`).

use super::*;

fn entries(n: usize) -> Vec<ListEntry> {
    (0..n)
        .map(|i| ListEntry::new(format!("id{i}"), format!("Item {i:05}")))
        .collect()
}

#[test]
fn a_hundred_thousand_entries_never_need_more_than_the_cap() {
    let e = entries(100_000);
    let rows = build_rows(&e, &[], &[], "", &[]);
    let l = RowLayout::new(&rows);
    let r = l.visible_range(0.0, 600.0);
    assert!(r.len() <= 20, "{r:?}");
    let mid = l.visible_range(l.total() / 2.0, 600.0);
    assert!(mid.len() <= 20 && mid.start > 40_000);
    let huge = l.visible_range(0.0, 1.0e9);
    assert!(huge.len() <= MAX_ROW_ENTITIES);
}

#[test]
fn sections_appear_only_without_a_query() {
    let mut e = entries(5);
    e[3].title = "Berlin".into();
    let rows = build_rows(&e, &["id3".into()], &["id1".into(), "id3".into()], "", &[]);
    assert_eq!(rows[0], Row::Header(Section::Recent));
    assert_eq!(rows[2], Row::Header(Section::Favourites));
    assert!(rows.contains(&Row::Header(Section::All)));
    let flat = build_rows(&e, &["id3".into()], &["id1".into()], "berl", &[]);
    assert_eq!(flat.len(), 1);
    assert_eq!(
        flat[0],
        Row::Item {
            entry: 3,
            section: Section::Results
        }
    );
}

#[test]
fn a_plain_list_has_no_headers() {
    let rows = build_rows(&entries(3), &[], &[], "", &[]);
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(Row::is_item));
}

#[test]
fn every_search_word_and_every_chip_must_match() {
    let e = [
        ListEntry::new("a", "MAN SD77")
            .with_subtitle("City bus")
            .with_tags(["road"]),
        ListEntry::new("b", "Strassenbahn").with_tags(["rail"]),
    ];
    assert!(e[0].matches("man 77", &[]));
    assert!(!e[0].matches("man tram", &[]));
    assert!(e[0].matches("", &["road".into()]));
    assert!(!e[1].matches("", &["road".into()]));
    assert!(e[0].matches("city", &[]), "the subtitle is searched");
}

#[test]
fn stepping_skips_headers_and_stops_at_the_ends() {
    let rows = build_rows(&entries(3), &["id1".into()], &[], "", &[]);
    // Header(Recent), Item(1), Header(All), Item 0, 1, 2
    assert_eq!(first_item(&rows), Some(1));
    assert_eq!(last_item(&rows), Some(5));
    assert_eq!(step_item(&rows, Some(1), 1), Some(3));
    assert_eq!(step_item(&rows, Some(3), -1), Some(1));
    assert_eq!(step_item(&rows, Some(1), -1), Some(1));
    assert_eq!(step_item(&rows, Some(5), 1), Some(5));
    assert_eq!(step_item(&rows, None, 1), Some(1));
    assert_eq!(step_item(&rows, Some(1), 4), Some(5));
}

#[test]
fn type_ahead_cycles_on_one_letter_and_refines_on_more() {
    let e = vec![
        ListEntry::new("1", "Aachen"),
        ListEntry::new("2", "Ahlheim"),
        ListEntry::new("3", "Berlin"),
    ];
    let rows = build_rows(&e, &[], &[], "", &[]);
    assert_eq!(type_ahead(&e, &rows, Some(0), "a"), Some(1));
    assert_eq!(type_ahead(&e, &rows, Some(1), "a"), Some(0), "wraps");
    assert_eq!(type_ahead(&e, &rows, Some(0), "ah"), Some(1));
    assert_eq!(type_ahead(&e, &rows, Some(0), "b"), Some(2));
    assert_eq!(type_ahead(&e, &rows, Some(0), "zz"), None);
}

#[test]
fn reveal_and_row_at_agree_with_mixed_heights() {
    let rows = build_rows(&entries(10), &["id2".into()], &[], "", &[]);
    let l = RowLayout::new(&rows);
    assert_eq!(l.top(1), HEADER_HEIGHT);
    assert_eq!(l.row_at(HEADER_HEIGHT + 1.0), 1);
    assert_eq!(l.reveal(0, 100.0, 200.0), 0.0);
    let r = l.reveal(8, 0.0, 200.0);
    assert_eq!(r, l.top(9) - 200.0);
    assert_eq!(l.reveal(8, r, 200.0), r, "already visible: no move");
}

#[test]
fn empty_lists_and_empty_viewports_are_empty_ranges() {
    let l = RowLayout::new(&[]);
    assert!(l.is_empty());
    assert_eq!(l.visible_range(0.0, 100.0), 0..0);
    let rows = build_rows(&entries(5), &[], &[], "", &[]);
    assert_eq!(RowLayout::new(&rows).visible_range(0.0, 0.0), 0..0);
}
