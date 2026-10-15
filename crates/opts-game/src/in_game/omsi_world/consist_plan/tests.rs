//! Placement tests on synthetic manifests: the invariant is that along the train every part's
//! rear coupling point meets the front coupling point of the next.

use std::collections::HashMap;

use super::*;

struct Fake(HashMap<String, BusManifest>);

impl Files for Fake {
    fn find_in(&self, _: &Path, rel: &str) -> Option<PathBuf> {
        let key = rel.to_lowercase();
        self.0
            .contains_key(&key)
            .then(|| PathBuf::from("/v").join(&key))
    }

    fn manifest(&self, path: &Path) -> Option<BusManifest> {
        self.0.get(path.file_name()?.to_str()?).cloned()
    }
}

/// A manifest with coupling points at `front` and `back` (y; x 0, z 1) and optionally a
/// `[couple_back]` link.
fn car(front: f32, back: f32, link: Option<(&str, bool)>) -> BusManifest {
    let mut t = format!("[coupling_front]\n0\n{front}\n1\n\n[coupling_back]\n0\n{back}\n1\n\n");
    if let Some((name, reversed)) = link {
        t.push_str(&format!("[couple_back]\n{name}\n{reversed}\n\n"));
    }
    BusManifest::parse(&t).expect("manifest")
}

fn files(cars: &[(&str, BusManifest)]) -> Fake {
    Fake(
        cars.iter()
            .map(|(n, m)| ((*n).to_owned(), m.clone()))
            .collect(),
    )
}

fn fcar(name: &str, turned: bool) -> FormationCar {
    FormationCar {
        path: PathBuf::from("/v").join(name),
        turned,
        powered: None,
    }
}

/// Rear and front coupling points of a part on the train's axis.
fn rear_front(m: &BusManifest, behind: f32, turned: bool) -> (f32, f32) {
    let (cf, cb) = (
        m.coupling_front.expect("front").y,
        m.coupling_back.expect("back").y,
    );
    if turned {
        (-behind - cf, -behind - cb)
    } else {
        (-behind + cb, -behind + cf)
    }
}

/// Every part's rear point meets the next part's front point.
fn assert_continuous(head: &BusManifest, plan: &Plan) {
    let mut prev = rear_front(head, 0.0, false).0;
    for (i, p) in plan.followers.iter().enumerate() {
        let (rear, front) = rear_front(&p.manifest, p.behind, p.turned);
        assert!(
            (front - prev).abs() < 1e-3,
            "part {i} ({}): front {front} does not meet the rear {prev} in front of it",
            p.path.display()
        );
        prev = rear;
    }
}

const NO_CARS: &[FormationCar] = &[];

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
