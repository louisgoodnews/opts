//! Validation tests for train formations: length, couplings, parts budget and reversing.

use std::collections::HashMap;

use super::super::validate::*;
use super::super::*;

struct Fixed(HashMap<&'static str, VehicleInfo>);

impl VehicleLookup for Fixed {
    fn info(&self, file: &str) -> Option<VehicleInfo> {
        self.0.get(file).cloned()
    }
}

fn car(cab: bool) -> VehicleInfo {
    VehicleInfo {
        is_rail: true,
        has_cab: cab,
        front_coupling: true,
        back_coupling: true,
        parts: 1,
    }
}

fn lookup() -> Fixed {
    let mut m = HashMap::new();
    m.insert("loco.ovh", car(true));
    m.insert("wagon.ovh", car(false));
    m.insert(
        "bus.ovh",
        VehicleInfo {
            is_rail: false,
            ..car(true)
        },
    );
    m.insert(
        "noback.ovh",
        VehicleInfo {
            back_coupling: false,
            ..car(false)
        },
    );
    m.insert(
        "nofront.ovh",
        VehicleInfo {
            front_coupling: false,
            ..car(false)
        },
    );
    m.insert(
        "unit.ovh",
        VehicleInfo {
            parts: 12,
            ..car(true)
        },
    );
    Fixed(m)
}

fn formation(cars: &[(&str, Orientation)]) -> TrainFormation {
    TrainFormation {
        name: "t".into(),
        segments: cars
            .iter()
            .map(|(f, o)| FormationSegment {
                vehicle: VehicleRef::File((*f).into()),
                orientation: *o,
                powered: None,
            })
            .collect(),
        source: FormationSource::Custom,
    }
}

use Orientation::{Backward, Forward};

#[test]
fn a_good_train_has_no_issue() {
    let f = formation(&[
        ("loco.ovh", Forward),
        ("wagon.ovh", Forward),
        ("wagon.ovh", Backward),
    ]);
    assert_eq!(f.validate(&lookup()), vec![]);
}

#[test]
fn empty_and_single_car_formations_are_too_short() {
    let l = lookup();
    assert_eq!(formation(&[]).validate(&l), vec![FormationIssue::TooShort]);
    assert_eq!(
        formation(&[("loco.ovh", Forward)]).validate(&l),
        vec![FormationIssue::TooShort]
    );
}

#[test]
fn missing_files_and_non_rail_vehicles_are_named_by_car() {
    let f = formation(&[
        ("loco.ovh", Forward),
        ("gone.ovh", Forward),
        ("bus.ovh", Forward),
    ]);
    assert_eq!(
        f.validate(&lookup()),
        vec![
            FormationIssue::Missing {
                index: 1,
                file: "gone.ovh".into()
            },
            FormationIssue::NotRail { index: 2 },
        ]
    );
}

#[test]
fn all_problems_are_reported_not_only_the_first() {
    let f = formation(&[
        ("wagon.ovh", Backward),
        ("bus.ovh", Forward),
        ("nofront.ovh", Forward),
    ]);
    let issues = f.validate(&lookup());
    assert!(issues.contains(&FormationIssue::NoCab));
    assert!(issues.contains(&FormationIssue::HeadTurned));
    assert!(issues.contains(&FormationIssue::NotRail { index: 1 }));
    assert!(issues.contains(&FormationIssue::Coupling {
        index: 2,
        end: End::Front
    }));
}

#[test]
fn the_coupling_needed_depends_on_the_neighbour_and_the_orientation() {
    let l = lookup();
    // `noback` is fine as the last car (nothing behind it) but not in the middle.
    let last = formation(&[("loco.ovh", Forward), ("noback.ovh", Forward)]);
    assert_eq!(last.validate(&l), vec![]);
    let middle = formation(&[
        ("loco.ovh", Forward),
        ("noback.ovh", Forward),
        ("wagon.ovh", Forward),
    ]);
    assert_eq!(
        middle.validate(&l),
        vec![FormationIssue::Coupling {
            index: 1,
            end: End::Rear
        }]
    );
    // Turned, its missing back point faces the front instead: fine in the middle only if the
    // front point is there for the rear side.
    let turned = formation(&[
        ("loco.ovh", Forward),
        ("noback.ovh", Backward),
        ("wagon.ovh", Forward),
    ]);
    assert_eq!(
        turned.validate(&l),
        vec![FormationIssue::Coupling {
            index: 1,
            end: End::Front
        }]
    );
}

#[test]
fn groups_cannot_be_spawned_but_turned_multi_file_cars_can() {
    let l = lookup();
    let mut f = formation(&[("loco.ovh", Forward), ("unit.ovh", Backward)]);
    assert_eq!(
        f.validate(&l),
        vec![],
        "a turned multi-file car is placed mirrored"
    );
    f.segments[1].vehicle = VehicleRef::Group("M-Train Cars".into());
    assert_eq!(f.validate(&l), vec![FormationIssue::Group { index: 1 }]);
}

#[test]
fn the_part_budget_counts_chain_parts() {
    let l = lookup();
    let mut cars = vec![("unit.ovh", Forward)];
    cars.extend(std::iter::repeat_n(("unit.ovh", Forward), 5)); // 6 x 12 = 72 parts
    let issues = formation(&cars).validate(&l);
    assert_eq!(issues, vec![FormationIssue::TooManyParts { parts: 72 }]);
    let ok = formation(&cars[..5]); // 60 parts
    assert_eq!(ok.validate(&l), vec![]);
}

#[test]
fn reversing_a_train_swaps_the_order_and_every_orientation() {
    let f = formation(&[
        ("loco.ovh", Forward),
        ("wagon.ovh", Backward),
        ("wagon.ovh", Forward),
    ]);
    let r = f.reversed();
    assert_eq!(r.segments[0].orientation, Orientation::Backward);
    assert_eq!(r.segments[2].vehicle.as_str(), "loco.ovh");
    assert_eq!(r.segments[2].orientation, Orientation::Backward);
    assert_eq!(r.segments[1].orientation, Orientation::Forward);
    assert_eq!(r.reversed(), f);
}

#[test]
fn issues_have_keys_cars_and_readable_text() {
    let i = FormationIssue::Coupling {
        index: 1,
        end: End::Rear,
    };
    assert_eq!(i.key(), "formation-err-coupling");
    assert_eq!(i.car(), Some(1));
    assert!(i.to_string().contains("car 2"));
    assert_eq!(FormationIssue::TooShort.car(), None);
}
