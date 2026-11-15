//! Tests of search, sections, recents, favourites and chips of the master-detail view.

use super::*;

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
