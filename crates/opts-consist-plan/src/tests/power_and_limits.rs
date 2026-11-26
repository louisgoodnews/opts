//! Power of a unit, the part limit, missing files, and the joints inside a unit.

use super::*;

#[test]
fn power_follows_the_unit_unless_told_otherwise() {
    let head = car(13.0, -12.0, None);
    let f = files(&[
        ("a.ovh", car(10.0, -9.0, Some(("b.ovh", false)))),
        ("b.ovh", car(6.0, -5.0, None)),
        ("w.ovh", car(9.0, -9.0, None)),
    ]);
    let mut cars = [fcar("a.ovh", false), fcar("w.ovh", false)];
    let p = plan(Path::new("/v"), &head, &cars, 64, &f);
    assert_eq!(
        p.followers.iter().map(|x| x.powered).collect::<Vec<_>>(),
        [true, true, false]
    );
    cars[0].powered = Some(false);
    cars[1].powered = Some(true);
    let p = plan(Path::new("/v"), &head, &cars, 64, &f);
    assert_eq!(
        p.followers.iter().map(|x| x.powered).collect::<Vec<_>>(),
        [false, false, true]
    );
}

#[test]
fn the_part_limit_and_missing_files_end_the_train() {
    let head = car(13.0, -12.0, None);
    let f = files(&[("w.ovh", car(9.0, -9.0, None))]);
    let cars = [
        fcar("w.ovh", false),
        fcar("w.ovh", false),
        fcar("w.ovh", false),
    ];
    assert_eq!(
        plan(Path::new("/v"), &head, &cars, 2, &f).followers.len(),
        2
    );
    assert_eq!(
        plan(Path::new("/v"), &head, &cars, 0, &f).followers.len(),
        0
    );
    let missing = [
        fcar("w.ovh", false),
        fcar("gone.ovh", false),
        fcar("w.ovh", false),
    ];
    assert_eq!(
        plan(Path::new("/v"), &head, &missing, 64, &f)
            .followers
            .len(),
        1,
        "the train ends before the car that cannot be read"
    );
}

#[test]
fn joints_inside_a_unit_are_the_leaders_rear_and_the_followers_front() {
    let head = car(13.0, -12.0, Some(("b.bus", false)));
    let f = files(&[("b.bus", car(10.0, -9.0, None))]);
    let plan = plan(Path::new("/v"), &head, NO_CARS, 40, &f);
    let (b, fr) = plan.followers[0].joint.expect("joint");
    assert_eq!((b.y, fr.y), (-12.0, 10.0));
}
