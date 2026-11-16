//! Turned cars and units, and a chain that loops back.

use super::*;

#[test]
fn a_turned_single_car_swaps_and_negates_its_points() {
    let head = car(13.0, -12.0, None);
    let f = files(&[
        ("w.ovh", car(11.0, -9.0, None)),
        ("x.ovh", car(7.0, -8.0, None)),
    ]);
    let cars = [fcar("w.ovh", true), fcar("x.ovh", false)];
    let plan = plan(Path::new("/v"), &head, &cars, 64, &f);
    assert!(plan.followers[0].turned);
    assert!(!plan.followers[1].turned);
    assert_continuous(&head, &plan);
}

#[test]
fn a_turned_chain_unit_comes_out_back_to_front() {
    let head = car(13.0, -12.0, None);
    let f = files(&[
        ("a.ovh", car(10.0, -9.0, Some(("b.ovh", false)))),
        ("b.ovh", car(6.0, -5.0, Some(("c.ovh", false)))),
        ("c.ovh", car(4.0, -3.0, None)),
        ("z.ovh", car(7.0, -8.0, None)),
    ]);
    let cars = [fcar("a.ovh", true), fcar("z.ovh", false)];
    let plan = plan(Path::new("/v"), &head, &cars, 64, &f);
    let names: Vec<String> = plan
        .followers
        .iter()
        .map(|p| p.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        ["c.ovh", "b.ovh", "a.ovh", "z.ovh"],
        "the unit is mirrored"
    );
    assert!(plan.followers[..3].iter().all(|p| p.turned));
    assert!(!plan.followers[3].turned);
    assert!(
        plan.followers[..3].iter().all(|p| p.powered),
        "a multiple unit pulls"
    );
    assert_continuous(&head, &plan);
    // Turned twice it is the unit as written again.
    let again = [fcar("a.ovh", false), fcar("z.ovh", false)];
    let plain = super::plan(Path::new("/v"), &head, &again, 64, &f);
    let names: Vec<String> = plain
        .followers
        .iter()
        .map(|p| p.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["a.ovh", "b.ovh", "c.ovh", "z.ovh"]);
    assert_continuous(&head, &plain);
}

#[test]
fn a_chain_that_loops_back_ends_its_unit_and_the_next_car_still_follows() {
    // `b` couples to `c` with the reversed flag (the rear cab of a tram seen from behind).
    let head = car(13.0, -12.0, Some(("b.ovh", false)));
    let f = files(&[
        ("b.ovh", car(10.0, -9.0, Some(("c.ovh", true)))),
        ("c.ovh", car(6.0, -5.0, Some(("b.ovh", false)))),
        ("w.ovh", car(9.0, -9.0, None)),
    ]);
    let cars = [fcar("w.ovh", false)];
    let plan = plan(Path::new("/v"), &head, &cars, 64, &f);
    let names: Vec<String> = plan
        .followers
        .iter()
        .map(|p| p.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        ["b.ovh", "c.ovh", "w.ovh"],
        "no endless loop, the car is attached"
    );
    assert!(plan.followers[1].turned, "the reversed link turns `c`");
    assert_continuous(&head, &plan);
}

