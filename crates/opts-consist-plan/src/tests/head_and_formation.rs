//! Placement of the head chain and of the formation cars behind it.

use super::*;

#[test]
fn the_head_chain_is_placed_by_the_old_rule() {
    let head = car(13.0, -12.0, Some(("b.bus", false)));
    let f = files(&[
        ("b.bus", car(10.0, -9.0, Some(("c.bus", false)))),
        ("c.bus", car(8.0, -7.0, None)),
    ]);
    let plan = plan(Path::new("/v"), &head, NO_CARS, 40, &f);
    let behind: Vec<f32> = plan.followers.iter().map(|p| p.behind).collect();
    // b: 10 + 12 = 22 behind the head; c: 22 + (8 + 9) = 39.
    assert_eq!(behind, [22.0, 39.0]);
    assert!(plan.followers.iter().all(|p| !p.turned && p.powered));
    assert_continuous(&head, &plan);
}

#[test]
fn formation_cars_follow_the_head_and_each_other() {
    let head = car(13.0, -12.0, None);
    let f = files(&[
        ("w.ovh", car(9.0, -9.0, None)),
        ("x.ovh", car(7.0, -8.0, None)),
    ]);
    let cars = [fcar("w.ovh", false), fcar("x.ovh", false)];
    let plan = plan(Path::new("/v"), &head, &cars, 64, &f);
    assert_eq!(plan.followers.len(), 2);
    assert_eq!(plan.followers[0].behind, 21.0); // 12 + 9
    assert_eq!(plan.followers[1].behind, 21.0 + 9.0 + 7.0);
    assert!(
        !plan.followers[0].powered,
        "a single-file car behind the head is hauled"
    );
    assert_continuous(&head, &plan);
}
