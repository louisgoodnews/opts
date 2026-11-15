//! Path tests: straight paths and arcs, and the fixture's roads near the centre.

use super::*;

#[test]
fn straight_paths_are_one_chord_and_arcs_follow_the_pixels() {
    let straight = MapRoad {
        geometry: PathGeometry::straight(bevy::math::DVec2::ZERO, 0.0, 500.0),
        width: 5.0,
    };
    let arc = MapRoad {
        geometry: PathGeometry {
            start: bevy::math::DVec2::ZERO,
            heading: 0.0,
            radius: 100.0,
            length: 300.0,
        },
        width: 5.0,
    };
    let count = |road: &MapRoad, min_chord: f64| {
        let mut out = Vec::new();
        push_chords(road, bevy::math::DVec2::ZERO, min_chord, &mut out);
        out.len()
    };
    assert_eq!(count(&straight, 5.0), 1);
    assert!(
        count(&arc, 5.0) > count(&arc, 50.0),
        "zoomed in: more pieces"
    );
    assert!(count(&arc, 0.01) <= 32, "bounded");
    let mut out = Vec::new();
    push_chords(&arc, bevy::math::DVec2::ZERO, 10.0, &mut out);
    for pair in out.windows(2) {
        assert!(pair[0].b.distance(pair[1].a) < 1e-3, "the chords join");
    }
}

#[test]
fn a_fixture_only_hands_over_roads_near_the_centre() {
    let fixture = FixtureRoads::grid();
    let mut near = Vec::new();
    fixture.roads_near(bevy::math::DVec2::ZERO, 100.0, &mut near);
    let mut far = Vec::new();
    fixture.roads_near(bevy::math::DVec2::new(5000.0, 5000.0), 100.0, &mut far);
    assert!(!near.is_empty());
    assert!(far.is_empty());
}
