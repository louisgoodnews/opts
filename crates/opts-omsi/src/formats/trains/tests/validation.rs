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

#[path = "validation/coupling.rs"]
mod coupling;
