//! Tests of the overview roads: which tiles feed the drawing, and the progress of the load.

use super::super::overview::OverviewRoads;
use crate::hud::minimap::MapRoad;
use bevy::math::DVec2;
use opts_traffic::PathGeometry;

fn road(x: f64, y: f64) -> MapRoad {
    MapRoad {
        geometry: PathGeometry::straight(DVec2::new(x, y), 0.0, 100.0),
        width: 5.0,
        on_course: false,
        ahead: false,
    }
}

#[test]
fn roads_come_from_the_tiles_near_the_view_and_not_from_skipped_ones() {
    let mut o = OverviewRoads::default();
    o.insert((0, 0), vec![road(0.0, 0.0)]);
    o.insert((5, 0), vec![road(5_000.0, 0.0)]);
    o.insert((9, 9), vec![road(9_000.0, 9_000.0)]);
    let mut out = Vec::new();
    o.roads_near(DVec2::new(50.0, 50.0), 300.0, |_| false, &mut out);
    assert_eq!(out.len(), 1, "only the tile at the origin is near");
    out.clear();
    o.roads_near(DVec2::new(50.0, 50.0), 300.0, |k| k == (0, 0), &mut out);
    assert!(
        out.is_empty(),
        "a resident tile is drawn from the graph, not twice"
    );
    out.clear();
    o.roads_near(DVec2::ZERO, 20_000.0, |_| false, &mut out);
    assert_eq!(out.len(), 3, "the whole map in view");
}

#[test]
fn progress_counts_tiles_with_and_without_roads() {
    let mut o = OverviewRoads::default();
    assert!(!o.started() && !o.complete());
    o.total = 4;
    assert_eq!(o.percent(), 0);
    o.insert((0, 0), vec![road(0.0, 0.0)]);
    o.insert((1, 0), Vec::new());
    assert_eq!(o.percent(), 50);
    assert_eq!(o.generation, 2, "every finished tile is a change to draw");
}
