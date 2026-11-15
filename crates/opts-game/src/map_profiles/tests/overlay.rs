//! Tests of the session overlay on GameSettings: it never reaches the file.


use super::*;


#[test]
fn a_session_overlay_never_reaches_the_file_and_restores_exactly() {
    let root = dir("overlay");
    std::fs::create_dir_all(&root).unwrap();
    let mut s = GameSettings::default();
    s.db_path_override = Some(root.join("settings.db"));
    s.set_raw("gameplay.traffic", "100");
    s.set_raw("texture_filtering", "16");
    s.save().unwrap();
    let before = s.clone();

    let mut values = BTreeMap::new();
    values.insert("gameplay.traffic".to_string(), "20".to_string());
    values.insert("texture_filtering".to_string(), "4".to_string());
    values.insert("graphics.bloom".to_string(), "true".to_string());
    s.overlay_session(&values);
    assert_eq!(s.raw("gameplay.traffic").as_deref(), Some("20"));
    assert_eq!(s.raw("graphics.bloom").as_deref(), Some("true"));
    assert_eq!(s.overlaid_keys().len(), 3);

    // Saving while the profile is on writes the global values.
    s.set_raw("ui_scale", "125");
    s.save().unwrap();
    let mut on_disk = GameSettings::default();
    let conn = rusqlite::Connection::open(root.join("settings.db")).unwrap();
    on_disk.apply_from_connection(&conn).unwrap();
    assert_eq!(on_disk.raw("gameplay.traffic").as_deref(), Some("100"));
    assert_eq!(on_disk.raw("texture_filtering").as_deref(), Some("16"));
    assert_eq!(on_disk.raw("graphics.bloom"), None);
    assert_eq!(
        on_disk.raw("ui_scale").as_deref(),
        Some("125"),
        "other changes still save"
    );

    // Restoring brings back the global values, including "nothing stored".
    s.restore_session();
    assert_eq!(s.raw("gameplay.traffic").as_deref(), Some("100"));
    assert_eq!(s.raw("texture_filtering").as_deref(), Some("16"));
    assert_eq!(s.raw("graphics.bloom"), None);
    assert!(s.overlaid_keys().is_empty());
    assert_eq!(s.raw("ui_scale").as_deref(), Some("125"));
    let _ = before;
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn overlaying_again_restores_the_earlier_overlay_first() {
    let mut s = GameSettings::default();
    s.set_raw("gameplay.traffic", "100");
    let mut a = BTreeMap::new();
    a.insert("gameplay.traffic".to_string(), "20".to_string());
    s.overlay_session(&a);
    let mut b = BTreeMap::new();
    b.insert("gameplay.pedestrians".to_string(), "5".to_string());
    s.overlay_session(&b);
    assert_eq!(s.raw("gameplay.traffic").as_deref(), Some("100"));
    assert_eq!(s.raw("gameplay.pedestrians").as_deref(), Some("5"));
    s.restore_session();
    assert_eq!(s.raw("gameplay.pedestrians"), None);
}
