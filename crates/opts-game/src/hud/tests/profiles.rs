//! Profile loading: shipped, legacy, broken and user profiles.

use super::*;

#[test]
fn every_widget_is_in_a_shipped_profile_and_the_profiles_are_clean() {
    let profiles = HudProfiles::load(None);
    assert_eq!(profiles.0.len(), VehicleKind::ALL.len());
    assert!(unplaced(&profiles).is_empty(), "{:?}", unplaced(&profiles));
    for kind in VehicleKind::ALL {
        let (p, problems) = HudProfile::parse(profile::shipped(kind)).expect("parses");
        assert_eq!(p.kind, kind);
        assert!(problems.is_empty(), "{}: {problems:?}", kind.id());
    }
    let mut ids: Vec<_> = WIDGETS.iter().map(|w| w.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), WIDGETS.len(), "widget ids are unique");
}

#[test]
fn the_tram_profile_has_no_rpm_and_shows_direction_and_lever_instead_of_brake_and_gear() {
    let profiles = HudProfiles::load(None);
    let tram = profiles.get(VehicleKind::Tram).expect("tram");
    let ids: Vec<_> = tram
        .in_row(widget::Row::Instruments)
        .iter()
        .map(|(_, w)| w.id)
        .collect();
    assert_eq!(ids, ["speed", "rail-direction", "rail-lever"]);
}

#[test]
fn profiles_written_for_the_old_slots_still_load_into_the_cluster() {
    let text = r#"HudProfile(kind: Bus, widgets: [
        (id: "speed", slot: Some(Instruments)), (id: "controls-hint", slot: Some(Hints)),
        (id: "alerts", slot: Some(Alerts)),
    ])"#;
    let (p, problems) = HudProfile::parse(text).expect("parses");
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(p.widgets.len(), 3);
    assert!(p
        .widgets
        .iter()
        .all(|e| e.slot == Some(Slot::ClusterBottomLeft)));
}

#[test]
fn bad_profile_entries_are_dropped_and_reported() {
    let text = r#"HudProfile(kind: Bus, widgets: [
        (id: "speed"), (id: "speed"), (id: "nope"), (id: "gear", slot: Some(MiniMap)),
    ])"#;
    let (p, problems) = HudProfile::parse(text).expect("parses");
    assert_eq!(p.widgets.len(), 1);
    assert!(problems.contains(&ProfileProblem::Duplicate("speed".into())));
    assert!(problems.contains(&ProfileProblem::UnknownWidget("nope".into())));
    assert!(problems.contains(&ProfileProblem::SlotNotAllowed(
        "gear".into(),
        Slot::MiniMap
    )));
    assert!(matches!(
        HudProfile::parse("not ron"),
        Err(ProfileProblem::Syntax(_))
    ));
}

#[test]
fn a_user_profile_replaces_the_shipped_one_and_a_broken_one_is_ignored() {
    let dir = std::env::temp_dir().join(format!("opts-hud-profiles-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(
        dir.join("bus.ron"),
        r#"HudProfile(kind: Bus, widgets: [(id: "speed")])"#,
    )
    .expect("write");
    std::fs::write(dir.join("tram.ron"), "garbage").expect("write");
    std::fs::write(
        dir.join("trolleybus.ron"),
        r#"HudProfile(kind: Bus, widgets: [])"#,
    )
    .expect("write");
    let profiles = HudProfiles::load(Some(&dir));
    assert_eq!(
        profiles.get(VehicleKind::Bus).map(|p| p.widgets.len()),
        Some(1)
    );
    assert!(profiles
        .get(VehicleKind::Tram)
        .is_some_and(|p| p.widgets.len() > 3));
    assert!(profiles
        .get(VehicleKind::Trolleybus)
        .is_some_and(|p| p.widgets.len() > 3));
    let _ = std::fs::remove_dir_all(&dir);
}
