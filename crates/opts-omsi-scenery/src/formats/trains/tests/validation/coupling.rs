//! Coupling, part-budget, reversing and issue-reporting checks of train formations.

use super::*;

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
