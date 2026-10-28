//! Tests for responsive layout, selection announcements and programmatic scroll/reveal on master-detail views.

use super::*;

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
    let offset = view(&app, &parts)
        .reveal_offset(&md, "id10")
        .expect("shown");
    assert!(offset < max, "id10 is near the top: {offset}");
    let mut md = md;
    let ok = app
        .world_mut()
        .get_mut::<MasterDetailView>(parts.root)
        .expect("view")
        .select_entry(&mut md, "id10");
    assert!(ok);
    *app.world_mut()
        .get_mut::<MasterDetail>(parts.root)
        .expect("model") = md;
    app.update();
    app.update();
    assert_eq!(model(&app, &parts).selected.as_deref(), Some("id10"));
    assert!(view(&app, &parts).scroll() <= offset + 1.0);
    // An entry that is not shown cannot be revealed or selected.
    let md = model(&app, &parts).clone();
    assert!(view(&app, &parts).reveal_offset(&md, "nope").is_none());
}
