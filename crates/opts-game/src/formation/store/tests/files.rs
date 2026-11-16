//! The store on disk: writes, reads, listing and deletion, ids that cannot escape the folder, and damaged files.

use super::*;

#[test]
fn the_store_writes_reads_lists_and_deletes() {
    let store = FormationStore::new(dir("basic"));
    assert!(store.list().is_empty(), "no folder yet");
    let id = store.save_new(&sample()).expect("saves");
    assert_eq!(id, "three-nf6d");
    assert_eq!(store.read(&id).expect("reads"), sample());
    assert!(store.dir().join("three-nf6d.toml").is_file());
    assert!(
        !store.dir().join("three-nf6d.toml.tmp").exists(),
        "the temporary file is renamed away"
    );
    let second = store.save_new(&sample()).expect("saves again");
    assert_eq!(second, "three-nf6d-2", "a taken id is never overwritten");
    store
        .write(&id, &TrainFormation::custom("replaced"))
        .expect("overwrites");
    assert_eq!(store.read(&id).expect("reads").name, "replaced");
    assert_eq!(store.list().len(), 2);
    store.delete(&id).expect("deletes");
    assert_eq!(store.read(&id), Err(StoreError::NotFound(id.clone())));
    assert_eq!(store.delete(&id), Err(StoreError::NotFound(id)));
    let _ = fs::remove_dir_all(store.dir());
}

#[test]
fn ids_cannot_escape_the_folder() {
    let store = FormationStore::new(dir("ids"));
    for bad in ["../x", "a/b", "", "a b", ".."] {
        assert_eq!(
            store.read(bad),
            Err(StoreError::InvalidName(bad.to_owned())),
            "{bad:?}"
        );
        assert!(store.write(bad, &sample()).is_err());
    }
}

#[test]
fn a_damaged_file_is_listed_with_its_error_and_kept() {
    let store = FormationStore::new(dir("damaged"));
    store.save_new(&sample()).expect("saves");
    fs::write(store.dir().join("broken.toml"), "format = [").expect("write");
    fs::write(store.dir().join("note.txt"), "not a formation").expect("write");
    let list = store.list();
    assert_eq!(list.len(), 2, "the .txt is ignored");
    let broken = list.iter().find(|s| s.id == "broken").expect("listed");
    assert!(broken.state.is_err());
    assert!(store.dir().join("broken.toml").is_file(), "never deleted");
    let _ = fs::remove_dir_all(store.dir());
}

#[test]
fn the_list_puts_the_newest_file_first() {
    let store = FormationStore::new(dir("order"));
    store
        .write("old", &TrainFormation::custom("old"))
        .expect("w");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    store
        .write("new", &TrainFormation::custom("new"))
        .expect("w");
    let ids: Vec<String> = store.list().into_iter().map(|s| s.id).collect();
    assert_eq!(ids, ["new", "old"]);
    let _ = fs::remove_dir_all(store.dir());
}
