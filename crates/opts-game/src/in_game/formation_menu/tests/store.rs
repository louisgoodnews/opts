//! Saving, loading and clearing trains through the store.

use super::*;

fn store(app: &App) -> FormationStore {
    app.world()
        .resource::<FormationLibrary>()
        .0
        .clone()
        .expect("store")
}

#[test]
fn save_writes_a_file_named_from_the_train_and_a_second_save_replaces_it() {
    let mut app = app();
    say(&mut app, &["add br218", "add zas", "rename My Train"]);
    say(&mut app, &["save"]);
    assert_eq!(ed(&app).saved_id.as_deref(), Some("my-train"));
    assert!(
        ed(&app).status.contains("Saved as my-train"),
        "{}",
        ed(&app).status
    );
    let ids: Vec<String> = store(&app).list().into_iter().map(|s| s.id).collect();
    assert_eq!(ids, ["my-train"]);
    say(&mut app, &["add by438", "save"]);
    let again = store(&app).list();
    assert_eq!(again.len(), 1, "saved over the same file, no my-train-2");
    assert_eq!(again[0].state.as_ref().expect("reads").len(), 3);
    let _ = std::fs::remove_dir_all(store(&app).dir());
}

#[test]
fn an_unnamed_train_is_saved_under_a_suggested_name_and_an_empty_one_is_refused() {
    let mut app = app();
    say(&mut app, &["save"]);
    assert!(ed(&app).status.contains("empty"), "{}", ed(&app).status);
    assert!(store(&app).list().is_empty());
    say(&mut app, &["add br218", "add zas", "add zas", "save"]);
    assert_eq!(ed(&app).formation().name, "BR218_OR + 2");
    assert_eq!(ed(&app).saved_id.as_deref(), Some("br218-or-2"));
    let _ = std::fs::remove_dir_all(store(&app).dir());
}

#[test]
fn a_saved_train_loads_back_and_a_clear_forgets_where_it_came_from() {
    let mut app = app();
    say(
        &mut app,
        &[
            "add br218",
            "add zas",
            "turn 2",
            "power 2",
            "rename Freight",
            "save",
        ],
    );
    say(&mut app, &["clear"]);
    assert!(ed(&app).formation().is_empty());
    assert_eq!(ed(&app).saved_id, None);
    {
        let store = store(&app);
        let mut e = app.world_mut().resource_mut::<FormationEditor>();
        actions::refresh_saved(&mut e, Some(&store));
        assert_eq!(e.saved.len(), 1);
        assert!(e.saved[0].1.contains("Freight"));
    }
    say(&mut app, &["load freight"]);
    let f = ed(&app).formation();
    assert_eq!(f.name, "Freight");
    assert_eq!(f.len(), 2);
    assert_eq!(f.segments[1].orientation, Orientation::Backward);
    assert_eq!(f.segments[1].powered, Some(true));
    assert_eq!(ed(&app).saved_id.as_deref(), Some("freight"));
    // Editing a loaded preset or saved train and saving again keeps the file.
    say(&mut app, &["remove 2", "save"]);
    assert_eq!(store(&app).list().len(), 1);
    // Deleting it removes the file and the link.
    {
        let store = store(&app);
        let mut e = app.world_mut().resource_mut::<FormationEditor>();
        actions::refresh_saved(&mut e, Some(&store));
    }
    say(&mut app, &["delete freight"]);
    assert!(store(&app).list().is_empty());
    assert_eq!(ed(&app).saved_id, None);
    let _ = std::fs::remove_dir_all(store(&app).dir());
}
