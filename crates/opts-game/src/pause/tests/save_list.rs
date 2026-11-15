//! The save list and loading a save.


use super::*;


fn set_name(app: &mut App, text: &str) {
    let entity = opts_gui::ui_tree::find_entity(app.world_mut(), "saves.name").expect("name field");
    app.world_mut()
        .get_mut::<opts_gui::components::text_field::M3TextField>(entity)
        .expect("field")
        .text = text.to_owned();
    app.update();
}

fn save_files(app: &App) -> usize {
    app.world()
        .resource::<crate::save::SaveStorage>()
        .0
        .as_ref()
        .map_or(0, |s| s.list().len())
}

#[test]
fn the_save_list_saves_asks_before_overwriting_renames_and_deletes() {
    let mut app = app();
    esc(&mut app);
    click(&mut app, "pause.save");
    let open = ids(&mut app);
    assert!(open.contains(&"saves.name".to_owned()), "{open:?}");
    assert!(
        !open.contains(&"pause.resume".to_owned()),
        "the overlay steps aside"
    );

    // An empty name is refused inline.
    click(&mut app, "saves.save");
    assert_eq!(save_files(&app), 0);
    assert!(app
        .world()
        .resource::<crate::save::ui::SavesUi>()
        .name_error
        .is_some());

    set_name(&mut app, "Erste Fahrt");
    click(&mut app, "saves.save");
    assert_eq!(save_files(&app), 1);
    assert!(ids(&mut app).contains(&"saves.load.erste-fahrt".to_owned()));

    // The same name asks; Cancel keeps one file and the old content.
    set_name(&mut app, "Erste Fahrt");
    click(&mut app, "saves.save");
    assert!(ids(&mut app).contains(&"dialog.saves.overwrite.cancel".to_owned()));
    click(&mut app, "dialog.saves.overwrite.cancel");
    assert_eq!(save_files(&app), 1);
    set_name(&mut app, "Erste Fahrt");
    click(&mut app, "saves.save");
    click(&mut app, "dialog.saves.overwrite.yes");
    assert_eq!(save_files(&app), 1);

    // Rename through the name field.
    click(&mut app, "saves.rename.erste-fahrt");
    set_name(&mut app, "Zweite");
    click(&mut app, "saves.rename-apply");
    let name = app
        .world()
        .resource::<crate::save::SaveStorage>()
        .0
        .as_ref()
        .and_then(|s| s.read("erste-fahrt").ok())
        .map(|l| l.file.meta.name);
    assert_eq!(name.as_deref(), Some("Zweite"));

    // Delete asks.
    click(&mut app, "saves.delete.erste-fahrt");
    assert!(ids(&mut app).contains(&"dialog.saves.delete.yes".to_owned()));
    click(&mut app, "dialog.saves.delete.yes");
    assert_eq!(save_files(&app), 0);
    assert!(!ids(&mut app).contains(&"saves.load.erste-fahrt".to_owned()));

    // Esc closes the list and the pause overlay returns.
    esc(&mut app);
    assert!(ids(&mut app).contains(&"pause.resume".to_owned()));
}

#[test]
fn loading_a_save_whose_map_is_missing_shows_the_error_template() {
    let mut app = app();
    esc(&mut app);
    click(&mut app, "pause.save");
    set_name(&mut app, "Karte weg");
    click(&mut app, "saves.save");
    let store = app
        .world()
        .resource::<crate::save::SaveStorage>()
        .0
        .clone()
        .expect("store");
    let mut loaded = store.read("karte-weg").expect("saved").file;
    loaded.sections.get_mut("session").expect("session").data["map_folder"] =
        "Gibt-es-nicht".into();
    store.write("karte-weg", &loaded).expect("edit");
    // Rebuild the list from the changed file.
    app.world_mut()
        .resource_mut::<crate::save::ui::SavesUi>()
        .dirty = true;
    app.update();
    click(&mut app, "saves.load.karte-weg");
    click(&mut app, "dialog.saves.load.yes");
    let shown = ids(&mut app);
    assert!(
        shown.contains(&"dialog.saves.error.another".to_owned()),
        "{shown:?}"
    );
    assert!(shown.contains(&"dialog.saves.error.back".to_owned()));
    assert_eq!(state(&app), AppState::InGame, "nothing was left");
    click(&mut app, "dialog.saves.error.another");
    let now = ids(&mut app);
    assert!(
        now.contains(&"saves.load.karte-weg".to_owned()),
        "the list stays: {now:?}"
    );
}
