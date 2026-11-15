//! Search tests: the reset on close, the localised label and id, and the wheel.

use super::*;

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
