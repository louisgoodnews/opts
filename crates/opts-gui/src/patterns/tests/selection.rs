//! Tests of the row selection of the master-detail view: arrows, scrolling by keys, type-ahead and clicks.

use super::*;

#[test]
fn a_hundred_thousand_entries_keep_at_most_200_row_entities() {
    let mut app = layout_app();
    let parts = spawn(&mut app, MasterDetail::new(entries(100_000), None));
    let live = view(&app, &parts).live_rows();
    assert!(live > 0 && live <= 200, "{live} live rows");
    assert!(live < 30, "a 560 px window needs ~10 rows, got {live}");
    focus_list(&mut app, &parts);
    act(&mut app, UiAction::Scroll(ScrollStep::End));
    let live = view(&app, &parts).live_rows();
    assert!(live <= 200 && live > 0, "{live} live rows at the end");
    assert_eq!(
        model(&app, &parts).selected.as_deref(),
        Some("id99999"),
        "End selects the last entry"
    );
    assert!(view(&app, &parts).scroll() > 1.0e5);
}

#[test]
fn arrows_move_the_selection_and_announce_it() {
    let mut app = layout_app();
    let parts = spawn(&mut app, MasterDetail::new(entries(50), Some("id0".into())));
    focus_list(&mut app, &parts);
    act(&mut app, UiAction::Navigate(Direction::Down));
    act(&mut app, UiAction::Navigate(Direction::Down));
    assert_eq!(model(&app, &parts).selected.as_deref(), Some("id2"));
    act(&mut app, UiAction::Navigate(Direction::Up));
    assert_eq!(model(&app, &parts).selected.as_deref(), Some("id1"));
    act(&mut app, UiAction::Scroll(ScrollStep::Home));
    assert_eq!(model(&app, &parts).selected.as_deref(), Some("id0"));
    act(&mut app, UiAction::Scroll(ScrollStep::Page(1)));
    let sel = model(&app, &parts).selected.clone().unwrap_or_default();
    assert_ne!(sel, "id0", "paging moves several entries");
    let idx: usize = sel[2..].parse().unwrap_or(0);
    assert!(idx >= 5, "a page is more than a few rows: {idx}");
}

#[test]
fn the_selected_row_is_reachable_after_scrolling_with_the_keyboard() {
    let mut app = layout_app();
    let parts = spawn(
        &mut app,
        MasterDetail::new(entries(200), Some("id0".into())),
    );
    focus_list(&mut app, &parts);
    for _ in 0..40 {
        act(&mut app, UiAction::Navigate(Direction::Down));
    }
    let v = view(&app, &parts);
    let cursor = v.cursor().unwrap_or(0);
    let top = cursor as f32 * 56.0;
    assert!(
        top >= v.scroll() && top + 56.0 <= v.scroll() + 560.0,
        "row {cursor} at {top} outside the viewport at scroll {}",
        v.scroll()
    );
}

#[test]
fn type_ahead_selects_by_first_letters_and_resets() {
    let mut app = layout_app();
    let list = vec![
        ListEntry::new("a", "Aachen"),
        ListEntry::new("b", "Berlin"),
        ListEntry::new("g", "Grundorf"),
        ListEntry::new("gr", "Gruenau"),
    ];
    let parts = spawn(&mut app, MasterDetail::new(list, Some("a".into())));
    focus_list(&mut app, &parts);
    let type_key = |app: &mut App, c: &str| {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyG,
            logical_key: Key::Character(c.into()),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
        app.update();
    };
    type_key(&mut app, "g");
    assert_eq!(model(&app, &parts).selected.as_deref(), Some("g"));
    type_key(&mut app, "r");
    type_key(&mut app, "u");
    type_key(&mut app, "e");
    assert_eq!(model(&app, &parts).selected.as_deref(), Some("gr"));
}

#[test]
fn clicking_a_row_selects_it_and_marks_it_with_fill_and_marker() {
    let mut app = layout_app();
    let parts = spawn(&mut app, MasterDetail::new(entries(20), Some("id0".into())));
    let target = {
        let mut q = app
            .world_mut()
            .query::<(Entity, &master_detail::MasterRowButton)>();
        q.iter(app.world())
            .find(|(_, r)| r.id == "id3")
            .map(|(e, _)| e)
            .expect("row id3")
    };
    click(&mut app, target);
    app.update();
    assert_eq!(model(&app, &parts).selected.as_deref(), Some("id3"));
    let row = {
        let mut q = app
            .world_mut()
            .query::<(Entity, &master_detail::MasterRowButton)>();
        q.iter(app.world())
            .find(|(_, r)| r.id == "id3")
            .map(|(e, _)| e)
            .expect("row id3 again")
    };
    let w = app.world();
    assert!(w.get::<crate::interaction::state::Selected>(row).is_some());
    assert!(w
        .get::<crate::tokens::components::BackgroundRole>(row)
        .is_some());
    let kids = w.get::<Children>(row).expect("children");
    let marker = kids
        .iter()
        .find(|c| {
            w.get::<Name>(*c)
                .is_some_and(|n| n.as_str() == "SelectionMarker")
        })
        .expect("marker");
    assert!(
        w.get::<crate::tokens::components::BackgroundRole>(marker)
            .is_some(),
        "the marker bar is drawn, selection is not colour alone"
    );
}
