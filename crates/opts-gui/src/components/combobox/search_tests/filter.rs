//! Search tests: visibility, threshold, the filter after the debounce and the no-match row.

use super::*;

#[test]
fn short_lists_stay_plain() {
    let (mut app, _) = app_with(8, None);
    let n = app
        .world_mut()
        .query_filtered::<Entity, With<ComboboxSearch>>()
        .iter(app.world())
        .count();
    assert_eq!(n, 0, "8 options is not more than the default threshold");
    let (mut app, _) = app_with(9, None);
    let n = app
        .world_mut()
        .query_filtered::<Entity, With<ComboboxSearch>>()
        .iter(app.world())
        .count();
    assert_eq!(n, 1);
}

#[test]
fn the_threshold_is_configurable() {
    let mut app = headless_app();
    {
        let mut commands = app.world_mut().commands();
        ComboboxBuilder::new("x")
            .with_options((0..4).map(|i| i.to_string()).collect())
            .with_search_threshold(3)
            .spawn(&mut commands);
    }
    app.update();
    app.update();
    let n = app
        .world_mut()
        .query_filtered::<Entity, With<ComboboxSearch>>()
        .iter(app.world())
        .count();
    assert_eq!(n, 1);
}

#[test]
fn opening_focuses_the_search_field_and_typing_filters_after_the_debounce() {
    let (mut app, combo) = app_with(16, None);
    open(&mut app, combo);
    let field = search_field(&mut app);
    let input = app.world().get::<FocusProxy>(field).unwrap().0;
    assert_eq!(focused(&mut app), Some(input), "opening focuses the field");
    assert_eq!(shown(&mut app).len(), 16);

    app.world_mut().get_mut::<M3TextField>(field).unwrap().text = "point 1".into();
    frames(&mut app, 2);
    assert_eq!(shown(&mut app).len(), 16, "within the 150 ms debounce");
    frames(&mut app, 3);
    assert_eq!(
        shown(&mut app),
        vec![1, 10, 11, 12, 13, 14, 15],
        "after the debounce"
    );
    assert!(!no_match_shown(&mut app));
}

#[test]
fn a_16_option_start_point_list_can_be_narrowed_to_one() {
    let (mut app, combo) = app_with(16, None);
    open(&mut app, combo);
    type_into(&mut app, "START POINT 7");
    frames(&mut app, 4);
    assert_eq!(shown(&mut app), vec![7]);
}

#[test]
fn no_match_shows_a_row_and_the_clear_button_restores_the_list() {
    let (mut app, combo) = app_with(16, Some(3));
    open(&mut app, combo);
    type_into(&mut app, "zzz");
    frames(&mut app, 4);
    assert!(shown(&mut app).is_empty());
    assert!(no_match_shown(&mut app));
    assert_eq!(state(&app, combo).1, Some(3), "the selection is kept");

    let clear = app
        .world_mut()
        .query_filtered::<(Entity, &TextFieldClear), ()>()
        .iter(app.world())
        .map(|(e, _)| e)
        .next()
        .expect("clear button");
    crate::testing::click(&mut app, clear);
    frames(&mut app, 3);
    assert_eq!(shown(&mut app).len(), 16);
    assert!(!no_match_shown(&mut app));
    assert_eq!(state(&app, combo), (true, Some(3)));
}
