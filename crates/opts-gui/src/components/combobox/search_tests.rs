//! Search tests of the combobox.

use super::build::ComboboxBuilder;
use super::search::{ComboboxList, ComboboxNoMatch, ComboboxSearch};
use super::*;
use crate::actions::{Direction, UiAction};
use crate::components::text_field::TextFieldClear;
use crate::testing::{act, focused, headless_app, wheel};
use std::time::Duration;

const STEP_MS: u64 = 50;

fn app_with(options: usize, selected: Option<usize>) -> (App, Entity) {
    app_with_id(options, selected, None)
}

fn app_with_id(options: usize, selected: Option<usize>, id: Option<&'static str>) -> (App, Entity) {
    let mut app = headless_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(STEP_MS),
    ));
    let opts: Vec<String> = (0..options).map(|i| format!("Start point {i}")).collect();
    let combo = {
        let mut commands = app.world_mut().commands();
        let mut b = ComboboxBuilder::new("Start").with_options(opts);
        if let Some(i) = selected {
            b = b.with_selected(i);
        }
        let e = b.spawn(&mut commands);
        if let Some(id) = id {
            commands.entity(e).insert(crate::ui_id::UiId::new(id));
        }
        e
    };
    for _ in 0..3 {
        app.update();
    }
    (app, combo)
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn open(app: &mut App, combo: Entity) {
    app.world_mut()
        .get_mut::<M3Combobox>(combo)
        .unwrap()
        .is_open = true;
    frames(app, 4);
}

fn search_field(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<ComboboxSearch>>()
        .single(app.world())
        .expect("a search field")
}

fn type_into(app: &mut App, text: &str) {
    let field = search_field(app);
    app.world_mut().get_mut::<M3TextField>(field).unwrap().text = text.to_string();
    frames(app, 2);
}

/// Indices of the options that are shown.
fn shown(app: &mut App) -> Vec<usize> {
    let mut q = app.world_mut().query::<(&ComboboxMenuItem, &Node)>();
    let mut v: Vec<usize> = q
        .iter(app.world())
        .filter(|(_, n)| n.display != Display::None)
        .map(|(i, _)| i.index)
        .collect();
    v.sort_unstable();
    v
}

fn no_match_shown(app: &mut App) -> bool {
    let mut q = app.world_mut().query::<(&ComboboxNoMatch, &Node)>();
    q.iter(app.world()).any(|(_, n)| n.display != Display::None)
}

fn state(app: &App, combo: Entity) -> (bool, Option<usize>) {
    let c = app.world().get::<M3Combobox>(combo).unwrap();
    (c.is_open, c.selected)
}

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

#[test]
fn closing_resets_the_filter() {
    let (mut app, combo) = app_with(16, None);
    open(&mut app, combo);
    type_into(&mut app, "point 1");
    frames(&mut app, 4);
    app.world_mut()
        .get_mut::<M3Combobox>(combo)
        .unwrap()
        .is_open = false;
    frames(&mut app, 4);
    assert_eq!(shown(&mut app).len(), 16);
}

#[test]
fn the_search_field_is_named_and_localised() {
    let (mut app, combo) = app_with(16, None);
    open(&mut app, combo);
    let field = search_field(&mut app);
    let f = app.world().get::<M3TextField>(field).unwrap();
    assert_eq!(f.label, "Filter the options");
    assert_eq!(f.placeholder, "Search...");
    app.world_mut()
        .resource_mut::<Localization>()
        .set_locale("de");
    frames(&mut app, 2);
    let f = app.world().get::<M3TextField>(field).unwrap();
    assert_eq!(f.label, "Optionen filtern");
    assert_eq!(f.placeholder, "Suchen ...");
    app.world_mut()
        .resource_mut::<Localization>()
        .set_locale("en-XA");
    frames(&mut app, 2);
    let f = app.world().get::<M3TextField>(field).unwrap();
    assert!(f.label.contains('é') || f.label.contains('í') || f.label.contains('ö'));
}

#[test]
fn the_search_field_has_a_stable_id_in_every_language() {
    let (mut app, combo) = app_with_id(16, None, Some("wizard.where.start-point"));
    open(&mut app, combo);
    frames(&mut app, 2);
    let field = search_field(&mut app);
    let id = |app: &App| {
        app.world()
            .get::<crate::ui_id::UiId>(field)
            .map(|i| i.0.to_string())
    };
    let en = id(&app);
    assert_eq!(
        en.as_deref(),
        Some("wizard.where.start-point/textbox:widget-combobox-search-label")
    );
    app.world_mut()
        .resource_mut::<Localization>()
        .set_locale("de");
    frames(&mut app, 3);
    assert_eq!(id(&app), en, "the id does not depend on the language");
}

#[test]
fn the_wheel_still_scrolls_the_list_and_stays_in_the_menu() {
    let (mut app, combo) = app_with(16, None);
    open(&mut app, combo);
    let menu = app
        .world_mut()
        .query_filtered::<Entity, With<ComboboxMenu>>()
        .single(app.world())
        .unwrap();
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<ComboboxList>>()
        .single(app.world())
        .unwrap();
    wheel(app.world_mut(), menu, 3.0);
    let y = app.world().get::<ScrollPosition>(list).unwrap().y;
    assert!(y > 0.0, "the wheel moved the list, got {y}");
    wheel(app.world_mut(), menu, -10.0);
    assert_eq!(app.world().get::<ScrollPosition>(list).unwrap().y, 0.0);
}
