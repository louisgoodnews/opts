//! Search tests: keyboard operation of a filtered list (arrows, Enter, Escape).

use super::*;

#[test]
fn the_selection_survives_filtering_that_keeps_it_visible() {
    let (mut app, combo) = app_with(16, Some(12));
    open(&mut app, combo);
    type_into(&mut app, "1");
    frames(&mut app, 4);
    assert!(shown(&mut app).contains(&12));
    assert_eq!(state(&app, combo).1, Some(12));
}

#[test]
fn arrow_down_enters_the_list_and_enter_picks() {
    let (mut app, combo) = app_with(16, None);
    open(&mut app, combo);
    type_into(&mut app, "point 13");
    act(&mut app, UiAction::Navigate(Direction::Down));
    frames(&mut app, 2);
    let item = focused(&mut app).expect("an option has focus");
    assert_eq!(
        app.world().get::<ComboboxMenuItem>(item).map(|i| i.index),
        Some(13),
        "Down skips the debounce and lands on the first match"
    );
    act(&mut app, UiAction::Confirm);
    assert_eq!(state(&app, combo), (false, Some(13)));
}

#[test]
fn enter_in_the_search_field_picks_the_first_match() {
    let (mut app, combo) = app_with(16, None);
    open(&mut app, combo);
    type_into(&mut app, "point 1");
    act(&mut app, UiAction::Confirm);
    frames(&mut app, 3);
    assert_eq!(state(&app, combo), (false, Some(1)));
}

#[test]
fn enter_with_no_match_does_nothing() {
    let (mut app, combo) = app_with(16, None);
    open(&mut app, combo);
    type_into(&mut app, "zzz");
    act(&mut app, UiAction::Confirm);
    frames(&mut app, 3);
    assert_eq!(state(&app, combo), (true, None));
}

#[test]
fn escape_clears_the_filter_first_then_closes() {
    let (mut app, combo) = app_with(16, None);
    open(&mut app, combo);
    type_into(&mut app, "point 1");
    frames(&mut app, 4);
    assert_eq!(shown(&mut app).len(), 7);
    act(&mut app, UiAction::Back);
    frames(&mut app, 2);
    assert!(state(&app, combo).0, "first Esc keeps the list open");
    assert_eq!(shown(&mut app).len(), 16, "and shows every option again");
    act(&mut app, UiAction::Back);
    assert!(!state(&app, combo).0, "second Esc closes it");
}
