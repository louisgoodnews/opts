//! The configuration file: the version field, a newer or damaged file, and a missing file.

use super::*;

#[test]
fn a_file_before_the_version_field_and_with_fields_missing_still_loads() {
    let d = dir("old");
    let path = d.join(FILE_NAME);
    std::fs::write(
        &path,
        r#"{"configurations":[{"id":"a","name":"A","vehicle":"citaro"}]}"#,
    )
    .expect("writes");
    let store = BusConfigurations::load_from(&path).expect("reads");
    assert_eq!(store.version, FORMAT_VERSION);
    let a = store.find("a").expect("listed");
    assert_eq!(
        (a.repaint.as_str(), a.setvars.len(), a.favourite, a.created),
        ("", 0, false, 0)
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_newer_file_is_reported_and_left_alone() {
    let d = dir("newer");
    let path = d.join(FILE_NAME);
    let text = r#"{"version":99,"configurations":[]}"#;
    std::fs::write(&path, text).expect("writes");
    assert_eq!(
        BusConfigurations::load_from(&path),
        Err(StoreProblem::Newer(99))
    );
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), text);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_damaged_file_is_reported_and_kept_aside_when_the_store_is_saved() {
    let d = dir("damaged");
    let path = d.join(FILE_NAME);
    std::fs::write(&path, "{ not json").expect("writes");
    assert!(matches!(
        BusConfigurations::load_from(&path),
        Err(StoreProblem::Corrupt(_))
    ));
    BusConfigurations::default().save_to(&path).expect("saves");
    assert_eq!(
        std::fs::read_to_string(d.join(format!("{FILE_NAME}.broken"))).expect("kept"),
        "{ not json"
    );
    assert!(BusConfigurations::load_from(&path).is_ok());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_missing_file_is_an_empty_store() {
    let d = dir("missing");
    let store = BusConfigurations::load_from(&d.join(FILE_NAME)).expect("empty");
    assert!(store.configurations.is_empty());
    let _ = std::fs::remove_dir_all(d);
}
