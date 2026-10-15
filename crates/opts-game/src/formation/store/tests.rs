//! Store tests: the schema, atomic writes, unique ids, damaged files.

use super::*;

fn dir(name: &str) -> PathBuf {
    let d = std::env::var_os("HOME")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join(".local/share/opts/benchmark/formation/store-tests")
        .join(format!("{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    d
}

fn sample() -> TrainFormation {
    TrainFormation {
        name: "Three NF6D".into(),
        segments: vec![
            FormationSegment {
                vehicle: VehicleRef::File("Vehicles\\Tram_NF6D\\NF6D_A.bus".into()),
                orientation: Orientation::Forward,
                powered: None,
            },
            FormationSegment {
                vehicle: VehicleRef::File("Vehicles\\Bm238\\By438_mi.ovh".into()),
                orientation: Orientation::Backward,
                powered: Some(false),
            },
            FormationSegment {
                vehicle: VehicleRef::Group("M-Train Cars".into()),
                orientation: Orientation::Forward,
                powered: Some(true),
            },
        ],
        source: FormationSource::ZugFile("Trains\\X10_112_IC-8.zug".into()),
    }
}

#[test]
fn the_schema_is_the_documented_one() {
    let text = encode(&sample()).expect("encodes");
    assert!(text.contains("format = 1"), "{text}");
    assert!(text.contains("name = \"Three NF6D\""));
    // The writer uses a TOML literal string (no escapes); the reader takes either form.
    assert!(
        text.contains("template = 'Trains\\X10_112_IC-8.zug'"),
        "{text}"
    );
    assert!(decode("format = 1\nname = \"x\"\ntemplate = \"Trains\\\\a.zug\"\n").is_ok());
    assert_eq!(text.matches("[[segment]]").count(), 3);
    assert!(text.contains("orientation = \"backward\""));
    assert!(text.contains("group = \"M-Train Cars\""));
    assert!(!text.contains("powered = \n"), "absent when automatic");
}

#[test]
fn a_formation_survives_the_round_trip_exactly() {
    let f = sample();
    assert_eq!(decode(&encode(&f).expect("encodes")).expect("decodes"), f);
    let custom = TrainFormation::custom("empty");
    assert_eq!(decode(&encode(&custom).expect("e")).expect("d"), custom);
}

#[test]
fn hand_written_files_load_with_defaults() {
    let f = decode("format = 1\nname = \"x\"\n\n[[segment]]\nvehicle = \"a.ovh\"\n\n[[segment]]\nvehicle = \"b.ovh\"\norientation = \"Backward\"\n")
        .expect("decodes");
    assert_eq!(f.segments[0].orientation, Orientation::Forward);
    assert_eq!(f.segments[1].orientation, Orientation::Backward);
    assert_eq!(f.source, FormationSource::Custom);
}

#[test]
fn bad_files_say_what_is_wrong() {
    for (text, needle) in [
        ("not toml [", "expected"),
        ("name = \"x\"\n", "format"),
        ("format = 0\nname = \"x\"\n", "format must be 1"),
        ("format = 1\nname = \"x\"\n[[segment]]\n", "exactly one"),
        (
            "format = 1\nname = \"x\"\n[[segment]]\nvehicle = \"a\"\ngroup = \"b\"\n",
            "exactly one",
        ),
        (
            "format = 1\nname = \"x\"\n[[segment]]\nvehicle = \"a\"\norientation = \"up\"\n",
            "orientation",
        ),
    ] {
        let e = decode(text).expect_err(text).to_string();
        assert!(e.to_lowercase().contains(needle), "{text:?} -> {e}");
    }
    assert_eq!(
        decode("format = 9\nname = \"x\"\n"),
        Err(StoreError::Newer(9))
    );
}

#[test]
fn too_many_cars_are_refused() {
    let mut t = String::from("format = 1\nname = \"x\"\n");
    for _ in 0..=MAX_CARS {
        t.push_str("[[segment]]\nvehicle = \"a.ovh\"\n");
    }
    assert!(matches!(decode(&t), Err(StoreError::Corrupt(m)) if m.contains("more than")));
}

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
