//! Unit tests for `super` (`registry.rs`).

use super::*;

fn params(name: &str) -> SpawnParams {
    SpawnParams {
        catalogue_id: name.to_lowercase(),
        name: name.to_owned(),
        bus_file: format!("Vehicles/{name}.bus"),
        ..Default::default()
    }
}

fn alive(reg: &mut SessionVehicles, name: &str, entity: u32) -> VehicleId {
    reg.register_alive(
        params(name),
        Entity::from_raw_u32(entity).expect("entity"),
        Some((7, format!("Entry {name}"))),
        Some(SpawnPoint {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            heading_deg: 90.0,
        }),
        None,
    )
}

#[test]
fn a_live_option_change_is_recorded_sorted_and_dropped_at_the_default() {
    let mut reg = SessionVehicles::default();
    let a = alive(&mut reg, "A", 1);
    assert!(reg.set_setvar(a, "Heater", 1, 0));
    assert!(reg.set_setvar(a, "display_ext", 4, 2));
    assert_eq!(
        reg.get(a).map(|r| r.setvars.clone()),
        Some(vec![
            ("Heater".to_owned(), 1),
            ("display_ext".to_owned(), 4)
        ])
    );
    // Back to the repaint's own value (any spelling of the name): the override goes.
    assert!(reg.set_setvar(a, "heater", 0, 0));
    assert_eq!(
        reg.get(a).map(|r| r.setvars.clone()),
        Some(vec![("display_ext".to_owned(), 4)])
    );
    assert!(!reg.set_setvar(99, "x", 1, 0));
}

#[test]
fn ids_are_stable_and_never_reused() {
    let mut reg = SessionVehicles::default();
    let a = alive(&mut reg, "A", 1);
    let b = alive(&mut reg, "B", 2);
    assert_eq!((a, b), (1, 2));
    reg.mark_despawned(b, DespawnReason::User);
    let c = alive(&mut reg, "C", 3);
    assert_eq!(c, 3, "a despawned id is not handed out again");
    assert_eq!(reg.get(b).map(|r| r.lifecycle), Some(Lifecycle::Despawned));
}

#[test]
fn the_successor_is_the_most_recent_other_living_vehicle() {
    let mut reg = SessionVehicles::default();
    let a = alive(&mut reg, "A", 1);
    let b = alive(&mut reg, "B", 2);
    let c = alive(&mut reg, "C", 3);
    reg.set_active(Some(c));
    assert_eq!(reg.successor_of(c), Some(b));
    reg.mark_despawned(b, DespawnReason::User);
    assert_eq!(reg.successor_of(c), Some(a));
    reg.mark_despawned(a, DespawnReason::User);
    assert_eq!(reg.successor_of(c), None);
}

#[test]
fn cycling_wraps_and_skips_the_gone() {
    let mut reg = SessionVehicles::default();
    let a = alive(&mut reg, "A", 1);
    let b = alive(&mut reg, "B", 2);
    let c = alive(&mut reg, "C", 3);
    reg.set_active(Some(a));
    assert_eq!(reg.cycle(1), Some(b));
    assert_eq!(reg.cycle(-1), Some(c));
    reg.mark_despawned(b, DespawnReason::Lost);
    reg.set_active(Some(a));
    assert_eq!(reg.cycle(1), Some(c));
    reg.set_active(None);
    assert_eq!(reg.cycle(1), Some(a));
    assert_eq!(reg.cycle(-1), Some(c));
    assert_eq!(SessionVehicles::default().cycle(1), None);
}

#[test]
fn spawn_requests_queue_in_order_and_skip_the_one_in_flight() {
    let mut reg = SessionVehicles::default();
    let a = reg.request_spawn(params("A"));
    let b = reg.request_spawn(params("B"));
    assert_eq!(reg.next_pending(None), Some(a));
    assert_eq!(reg.next_pending(Some(a)), Some(b));
    reg.mark_failed(a, "no such file");
    assert_eq!(reg.next_pending(None), Some(b));
    assert_eq!(
        reg.get(a).and_then(|r| r.error.as_deref()),
        Some("no such file")
    );
}

#[test]
fn the_registry_round_trips_as_a_save_section() {
    let mut reg = SessionVehicles::default();
    let a = alive(&mut reg, "A", 1);
    let b = alive(&mut reg, "B", 2);
    reg.mark_despawned(a, DespawnReason::User);
    reg.set_active(Some(b));
    let section = reg.to_section();
    assert_eq!(section["version"], SAVE_SECTION_VERSION);
    assert_eq!(section["vehicles"].as_array().map(Vec::len), Some(1));
    assert!(
        section["vehicles"][0].get("entity").is_none(),
        "runtime entities are not saved"
    );
    let back = SessionVehicles::from_section(&section).expect("round trip");
    assert_eq!(back.active_id(), Some(b));
    let r = back.get(b).expect("record");
    assert_eq!(r.spawn_entry_name, "Entry B");
    assert_eq!(
        r.lifecycle,
        Lifecycle::Spawning,
        "restored vehicles are rebuilt"
    );
    assert_eq!(r.entity, None);
    let c = {
        let mut back = back;
        back.request_spawn(params("C"))
    };
    assert!(c > b, "ids continue after a restore");
    assert!(SessionVehicles::from_section(&json!({ "version": 99 })).is_err());
    assert!(
        SessionVehicles::from_section(&json!({ "version": 1 })).is_err(),
        "an old version goes through the migration first"
    );
}
