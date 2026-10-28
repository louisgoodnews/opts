//! Unit tests for `super` (`store.rs`).

use super::*;
use crate::save::container::SectionBlob;
use serde_json::json;

fn dir(name: &str) -> PathBuf {
    let d = std::env::var_os("HOME")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join(".local/share/opts/benchmark/session-save/store-tests")
        .join(name);
    let _ = fs::remove_dir_all(&d);
    d
}

fn file(name: &str, value: i64) -> SaveFile {
    let mut f = SaveFile::new(SaveMeta {
        name: name.to_owned(),
        saved_unix: value,
        ..Default::default()
    });
    f.sections.insert(
        "s".into(),
        SectionBlob {
            version: 1,
            data: json!({ "v": value }),
        },
    );
    f
}

#[test]
fn write_read_list_rename_delete() {
    let store = SaveStore::new(dir("basic"));
    let id = store.unique_id("Erste Fahrt");
    assert_eq!(id, "erste-fahrt");
    store.write(&id, &file("Erste Fahrt", 10)).expect("write");
    assert_eq!(store.unique_id("Erste Fahrt"), "erste-fahrt-2");
    store.write("zweite", &file("Zweite", 20)).expect("write");
    let list = store.list();
    assert_eq!(
        list.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
        ["zweite", "erste-fahrt"],
        "the most recent first"
    );
    store.rename("zweite", "  Neu  ").expect("rename");
    assert_eq!(store.read("zweite").expect("read").file.meta.name, "Neu");
    assert_eq!(store.find("neu").as_deref(), Some("zweite"));
    assert_eq!(store.find("erste-fahrt").as_deref(), Some("erste-fahrt"));
    store.delete("zweite").expect("delete");
    assert!(matches!(store.read("zweite"), Err(SaveError::NotFound(_))));
    assert!(matches!(
        store.delete("zweite"),
        Err(SaveError::NotFound(_))
    ));
    assert!(matches!(store.read("../x"), Err(SaveError::InvalidName(_))));
}

#[test]
fn an_overwrite_keeps_the_previous_version_as_the_backup() {
    let store = SaveStore::new(dir("backup"));
    store.write("a", &file("A", 1)).expect("first");
    store.write("a", &file("A", 2)).expect("second");
    assert_eq!(store.read("a").expect("read").file.meta.saved_unix, 2);
    let bak = SaveFile::decode(&fs::read_to_string(store.with_suffix("a", "bak")).expect("bak"))
        .expect("bak decodes");
    assert_eq!(bak.meta.saved_unix, 1);
}

#[test]
fn a_crash_while_writing_leaves_the_save_untouched() {
    let store = SaveStore::new(dir("partial"));
    store.write("a", &file("A", 1)).expect("write");
    // The process died half way through step 1: a truncated temporary file.
    let text = file("A", 2).encode().expect("encode");
    store
        .write_tmp("a", &text.as_bytes()[..text.len() / 2])
        .expect("tmp");
    let loaded = store.read("a").expect("read");
    assert_eq!(loaded.file.meta.saved_unix, 1);
    assert!(!loaded.from_backup);
    // The list ignores the leftover and the next write replaces it.
    assert_eq!(store.list().len(), 1);
    store.write("a", &file("A", 3)).expect("write again");
    assert_eq!(store.read("a").expect("read").file.meta.saved_unix, 3);
    assert!(!store.with_suffix("a", "tmp").exists());
}

#[test]
fn a_crash_between_the_renames_is_recovered_from_the_backup() {
    let store = SaveStore::new(dir("between"));
    store.write("a", &file("A", 1)).expect("write");
    // Step 2 happened, step 3 did not: only `.bak` and `.tmp` exist.
    fs::rename(store.main_path("a"), store.with_suffix("a", "bak")).expect("rotate");
    store
        .write_tmp("a", file("A", 2).encode().expect("encode").as_bytes())
        .expect("tmp");
    let loaded = store.read("a").expect("recovered");
    assert!(loaded.from_backup);
    assert_eq!(loaded.file.meta.saved_unix, 1);
    assert_eq!(store.list().len(), 1);
}

#[test]
fn a_damaged_main_file_falls_back_to_the_backup_and_without_one_reports_the_damage() {
    let store = SaveStore::new(dir("corrupt"));
    store.write("a", &file("A", 1)).expect("write");
    store.write("a", &file("A", 2)).expect("write");
    fs::write(store.main_path("a"), "{ not json").expect("damage");
    let loaded = store.read("a").expect("backup");
    assert!(loaded.from_backup);
    assert_eq!(loaded.file.meta.saved_unix, 1);
    fs::remove_file(store.with_suffix("a", "bak")).expect("remove backup");
    assert!(matches!(store.read("a"), Err(SaveError::Corrupt(_))));
    let list = store.list();
    assert_eq!(list.len(), 1);
    assert!(
        list[0].state.is_err(),
        "a damaged save is listed with its error"
    );
}

#[test]
fn a_newer_file_is_not_replaced_by_the_backup() {
    let store = SaveStore::new(dir("newer"));
    store.write("a", &file("A", 1)).expect("write");
    store.write("a", &file("A", 2)).expect("write");
    fs::write(
        store.main_path("a"),
        json!({"magic": crate::save::container::MAGIC, "format_version": 7}).to_string(),
    )
    .expect("newer");
    assert!(matches!(
        store.read("a"),
        Err(SaveError::NewerFormat { found: 7, .. })
    ));
}

#[test]
fn a_huge_file_is_refused_before_it_is_read() {
    let store = SaveStore::new(dir("huge"));
    fs::create_dir_all(store.dir()).expect("dir");
    let f = fs::File::create(store.main_path("big")).expect("create");
    f.set_len(MAX_FILE_BYTES + 1).expect("sparse");
    assert!(matches!(store.read("big"), Err(SaveError::TooLarge(_))));
}
