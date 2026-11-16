//! Names and identities of saved configurations: unique ids, replacing under a taken name, rename and delete.

use super::*;

#[test]
fn saving_under_a_taken_name_replaces_and_keeps_identity() {
    let mut store = BusConfigurations::default();
    let mut first = BusConfiguration::new("Day", "citaro", "", Vec::new(), 10);
    first.notes = "line 33".into();
    let (id, _) = store.save(first);
    store.set_favourite(&id, true);
    let (again, replaced) =
        store.save(BusConfiguration::new("DAY", "lions", "Red", Vec::new(), 50));
    assert!(replaced);
    assert_eq!(again, id);
    assert_eq!(store.configurations.len(), 1);
    let c = &store.configurations[0];
    assert_eq!(
        (c.vehicle.as_str(), c.created, c.last_used),
        ("lions", 10, 50)
    );
    assert_eq!(c.notes, "line 33");
    assert!(c.favourite);
}

#[test]
fn ids_are_unique_slugs() {
    let mut store = BusConfigurations::default();
    let mut add = |name: &str| {
        store
            .save(BusConfiguration::new(name, "citaro", "", Vec::new(), 1))
            .0
    };
    assert_eq!(add("Früh / Spät"), "früh-spät");
    // A different name with the same slug is numbered; the same name ignoring case replaces.
    assert_eq!(add("Früh - Spät"), "früh-spät-2");
    assert_eq!(add("FRÜH / SPÄT"), "früh-spät");
    assert_eq!(add("///"), "configuration");
    assert_eq!(store.configurations.len(), 3);
}

#[test]
fn rename_duplicate_and_delete() {
    let mut store = BusConfigurations::default();
    store.save(BusConfiguration::new("A", "citaro", "", Vec::new(), 1));
    store.save(BusConfiguration::new("B", "lions", "", Vec::new(), 1));
    assert!(!store.rename("a", "b"), "name taken");
    assert!(!store.rename("a", "  "), "empty name");
    assert!(store.rename("a", "Alpha"));
    assert_eq!(store.find("alpha").map(|c| c.id.as_str()), Some("a"));
    assert_eq!(
        store.duplicate("alpha", "Alpha copy", 9).as_deref(),
        Some("alpha-copy")
    );
    assert!(
        store.duplicate("alpha", "alpha COPY", 9).is_none(),
        "name taken"
    );
    assert!(store.duplicate("missing", "X", 9).is_none());
    assert!(store.delete("alpha-copy"));
    assert!(!store.delete("alpha-copy"));
    assert_eq!(store.configurations.len(), 2);
}
