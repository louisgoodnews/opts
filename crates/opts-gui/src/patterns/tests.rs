//! Tests of the master-detail pattern in Bevy's real UI layout.

use super::*;
use crate::actions::{Direction, ScrollStep, UiAction};
use crate::interaction::focus::FocusEvent;
use crate::l10n::{wizard_keys::md, Localization};
use crate::testing::{act, click, layout_app};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;

fn entries(n: usize) -> Vec<ListEntry> {
    (0..n)
        .map(|i| ListEntry::new(format!("id{i}"), format!("Entry {i:05}")).with_glyph(glyphs::MAP))
        .collect()
}

fn config() -> MasterDetailConfig {
    MasterDetailConfig {
        prefix: "t",
        empty_key: md::NO_RESULTS,
        search_key: md::SEARCH,
    }
}

fn spawn(app: &mut App, model: MasterDetail) -> MasterDetailParts {
    let loc = app.world().resource::<Localization>().clone();
    let parts = {
        let mut commands = app.world_mut().commands();
        let parts = spawn_master_detail(&mut commands, &loc, config(), model);
        let frame = commands
            .spawn(Node {
                width: Val::Px(1000.0),
                height: Val::Px(560.0),
                ..default()
            })
            .id();
        commands.entity(frame).add_child(parts.root);
        parts
    };
    for _ in 0..6 {
        app.update();
    }
    parts
}

fn view<'a>(app: &'a App, parts: &MasterDetailParts) -> &'a MasterDetailView {
    app.world()
        .get::<MasterDetailView>(parts.root)
        .expect("view")
}

fn model<'a>(app: &'a App, parts: &MasterDetailParts) -> &'a MasterDetail {
    app.world().get::<MasterDetail>(parts.root).expect("model")
}

fn focus_list(app: &mut App, parts: &MasterDetailParts) {
    app.world_mut().write_message(FocusEvent::Set(parts.list));
    app.update();
    app.update();
}

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

#[test]
fn search_filters_keeps_the_selection_and_clear_filter_restores() {
    let mut app = layout_app();
    let list = vec![
        ListEntry::new("a", "Aachen"),
        ListEntry::new("b", "Berlin"),
        ListEntry::new("c", "Chicago"),
    ];
    let parts = spawn(&mut app, MasterDetail::new(list, Some("b".into())));
    app.world_mut()
        .get_mut::<MasterDetail>(parts.root)
        .expect("model")
        .query = "zzz".into();
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(view(&app, &parts).live_rows(), 0);
    assert_eq!(model(&app, &parts).selected.as_deref(), Some("b"));
    let clear = {
        let mut q = app
            .world_mut()
            .query::<(Entity, &master_detail::MasterClearButton)>();
        q.iter(app.world()).next().map(|(e, _)| e).expect("clear")
    };
    click(&mut app, clear);
    for _ in 0..3 {
        app.update();
    }
    assert!(model(&app, &parts).query.is_empty());
    assert_eq!(view(&app, &parts).live_rows(), 3);
}

#[test]
fn recents_and_favourites_get_sections_and_the_star_toggles() {
    let mut app = layout_app();
    let mut m = MasterDetail::new(entries(5), Some("id0".into()));
    m.recents = vec!["id2".into()];
    m.favourites = vec!["id4".into()];
    let parts = spawn(&mut app, m);
    let rows = view(&app, &parts).rows().to_vec();
    assert!(matches!(
        rows[0],
        list_model::Row::Header(list_model::Section::Recent)
    ));
    let star = {
        let mut q = app
            .world_mut()
            .query::<(Entity, &master_detail::MasterStarButton)>();
        q.iter(app.world())
            .find(|(_, s)| s.id == "id1")
            .map(|(e, _)| e)
            .expect("star id1")
    };
    click(&mut app, star);
    app.update();
    assert!(model(&app, &parts).favourites.contains(&"id1".to_string()));
    assert!(
        model(&app, &parts).selected.as_deref() == Some("id0"),
        "a star click never changes the selection"
    );
}

#[test]
fn chips_filter_by_tag() {
    let mut app = layout_app();
    let list = vec![
        ListEntry::new("a", "Bus A").with_tags(["road"]),
        ListEntry::new("b", "Tram B").with_tags(["rail"]),
    ];
    let mut m = MasterDetail::new(list, Some("a".into()));
    m.chips = vec![Chip {
        tag: "rail".into(),
        label: "Rail".into(),
    }];
    let parts = spawn(&mut app, m);
    let chip = {
        let mut q = app
            .world_mut()
            .query::<(Entity, &master_detail::MasterChipButton)>();
        q.iter(app.world()).next().map(|(e, _)| e).expect("chip")
    };
    click(&mut app, chip);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(model(&app, &parts).active_chips, vec!["rail".to_string()]);
    assert_eq!(view(&app, &parts).live_rows(), 1);
}

fn set_window_width(app: &mut App, width: f32) {
    let mut q = app.world_mut().query::<&mut Window>();
    for mut w in q.iter_mut(app.world_mut()) {
        w.resolution.set(width, 600.0);
    }
}

#[test]
fn compact_width_stacks_list_above_detail() {
    let mut app = layout_app();
    let parts = spawn(&mut app, MasterDetail::new(entries(3), None));
    set_window_width(&mut app, 800.0);
    for _ in 0..3 {
        app.update();
    }
    let n = app.world().get::<Node>(parts.root).expect("root node");
    assert_eq!(n.flex_direction, FlexDirection::Column);
    set_window_width(&mut app, 1920.0);
    for _ in 0..3 {
        app.update();
    }
    let n = app.world().get::<Node>(parts.root).expect("root node");
    assert_eq!(n.flex_direction, FlexDirection::Row);
}

#[derive(Resource, Default)]
struct Seen(Vec<Option<String>>);

#[test]
fn the_selection_is_announced_at_the_start_and_on_every_change() {
    let mut app = layout_app();
    app.init_resource::<Seen>().add_systems(
        Update,
        |mut r: MessageReader<MasterDetailSelected>, mut seen: ResMut<Seen>| {
            seen.0.extend(r.read().map(|m| m.id.clone()));
        },
    );
    let parts = spawn(&mut app, MasterDetail::new(entries(3), Some("id1".into())));
    assert_eq!(
        app.world().resource::<Seen>().0,
        vec![Some("id1".to_string())]
    );
    focus_list(&mut app, &parts);
    act(&mut app, UiAction::Navigate(Direction::Down));
    assert_eq!(
        app.world().resource::<Seen>().0,
        vec![Some("id1".to_string()), Some("id2".to_string())]
    );
}

#[test]
fn a_selection_far_down_is_scrolled_into_view_when_the_list_appears() {
    let mut app = layout_app();
    let parts = spawn(
        &mut app,
        MasterDetail::new(entries(500), Some("id400".into())),
    );
    let v = view(&app, &parts);
    let cursor = v.cursor().expect("cursor");
    let top = cursor as f32 * 56.0;
    assert!(
        top >= v.scroll() && top + 56.0 <= v.scroll() + 560.0,
        "row {cursor} at {top} not visible at scroll {}",
        v.scroll()
    );
    assert!(v.scroll() > 20_000.0);
}

#[test]
fn programmatic_scroll_reveal_and_select_do_what_the_wheel_and_a_click_do() {
    let mut app = layout_app();
    let parts = spawn(&mut app, MasterDetail::new(entries(500), None));
    let (viewport, max) = {
        let v = view(&app, &parts);
        (v.viewport(), v.max_scroll())
    };
    assert!(viewport > 100.0 && max > viewport, "{viewport} / {max}");
    // Scrolling is clamped to the content, like the wheel.
    app.world_mut()
        .get_mut::<MasterDetailView>(parts.root)
        .expect("view")
        .scroll_by(-500.0);
    assert_eq!(view(&app, &parts).scroll(), 0.0);
    app.world_mut()
        .get_mut::<MasterDetailView>(parts.root)
        .expect("view")
        .scroll_to(f32::MAX);
    assert_eq!(view(&app, &parts).scroll(), max);
    // The listing covers every entry, not only the rows that exist as entities.
    let shown = view(&app, &parts).shown_entries(model(&app, &parts)).len();
    assert_eq!(shown, 500);
    assert!(view(&app, &parts).live_rows() < 60);
    // Revealing an entry and selecting it, as a click on its row would.
    let md = model(&app, &parts).clone();
    let offset = view(&app, &parts).reveal_offset(&md, "id10").expect("shown");
    assert!(offset < max, "id10 is near the top: {offset}");
    let mut md = md;
    let ok = app
        .world_mut()
        .get_mut::<MasterDetailView>(parts.root)
        .expect("view")
        .select_entry(&mut md, "id10");
    assert!(ok);
    *app.world_mut().get_mut::<MasterDetail>(parts.root).expect("model") = md;
    app.update();
    app.update();
    assert_eq!(model(&app, &parts).selected.as_deref(), Some("id10"));
    assert!(view(&app, &parts).scroll() <= offset + 1.0);
    // An entry that is not shown cannot be revealed or selected.
    let md = model(&app, &parts).clone();
    assert!(view(&app, &parts).reveal_offset(&md, "nope").is_none());
}
