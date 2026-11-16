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

mod head_and_formation;
mod power_and_limits;
mod units;
