//! Density tests of the traffic spawner.

use super::*;
use opts_core::formats::map::GroupDensity;

fn group(name: &str, density_group: usize, density: Option<GroupDensity>) -> PoolGroup {
    PoolGroup {
        name: name.into(),
        density,
        density_group,
    }
}

/// Berlin-Spandau's groups: NormalCars 1, Trucks 0, Commercials 1, Ambulance 1, GDRCars 0.
const SPANDAU: [u32; 5] = [1, 0, 1, 1, 0];

fn lane(x: f64, rules: &[(usize, f64)]) -> LaneSegment {
    let mut seg = LaneSegment::straight(DVec2::new(x, 0.0), 0.0, 100.0, LaneKind::Road);
    for &(g, v) in rules {
        seg.rules.density.set(g, v);
    }
    seg
}

/// GDRCars (default 0) drive only on lanes whose rule has flag 4; values above 1 count fully.
#[test]
fn group_density_comes_from_the_rule_flag() {
    let graph = LaneGraph::build(
        &[
            lane(0.0, &[(0, 1.0)]),
            lane(10.0, &[(0, 0.0), (4, 3.0)]),
            lane(20.0, &[]),
            lane(30.0, &[(1, 1.0)]),
        ],
        &ConnectParams::default(),
    );
    let groups = [
        group("NormalCars", 0, None),
        group("Trucks", 1, None),
        group("Commercials", 2, None),
        group("GDRCars", 4, None),
    ];
    let m = group_lane_metres(&graph, DVec2::new(15.0, 50.0), 1000.0, &groups, &SPANDAU);
    // NormalCars: rule 1 + none (Medium 1) + none on the trucks lane (Medium 1); closed by 0.
    assert!((m[0] - 300.0).abs() < 1e-6, "{m:?}");
    assert!(
        (m[1] - 100.0).abs() < 1e-6,
        "Trucks only where a flag-1 rule is: {m:?}"
    );
    assert!(
        (m[2] - m[0]).abs() < 1e-6,
        "Commercials follow NormalCars: {m:?}"
    );
    assert!(
        (m[3] - 300.0).abs() < 1e-6,
        "GDRCars: flag-4 lane at 3x, nowhere else: {m:?}"
    );
}

/// Only the part of a lane inside the disc counts.
#[test]
fn lane_metres_count_inside_the_disc_only() {
    let graph = LaneGraph::build(&[lane(0.0, &[])], &ConnectParams::default());
    let m = group_lane_metres(
        &graph,
        DVec2::new(0.0, 0.0),
        49.0,
        &[group("N", 0, None)],
        &[1],
    );
    assert!(m[0] > 30.0 && m[0] < 60.0, "{m:?}");
}

/// Time factors: the group's own curve (hour wrapping, weekend curve), else the map's road curve.
#[test]
fn time_factors_wrap_and_follow_the_day() {
    let own = opts_core::formats::map::parse_unsched_trafficdens(
        b"[group]\nNormalCars\n0.6\n[set_day_of_week]\n1\n[trafficdensity]\n0\n0.1\n[trafficdensity]\n12\n1.5\n[trafficdensity]\n24\n0.1\n[set_day_of_week]\n6\n[trafficdensity]\n0\n0.2\n[trafficdensity]\n24\n0.2\n",
    );
    let g = group("NormalCars", 0, own.into_iter().next());
    let road = DensityProfile {
        days: 0,
        points: vec![(0.0, 0.5), (24.0, 0.5)],
    };
    let f = |day, h| time_factor(&g, &road, day, h);
    assert!(
        (f(day::WEEKDAY, 12.0) - 0.9).abs() < 1e-6,
        "0.6 × 1.5, unclamped curve"
    );
    assert!((f(day::WEEKDAY, 36.0) - f(day::WEEKDAY, 12.0)).abs() < 1e-6);
    assert!((f(day::SUNDAY, 12.0) - 0.12).abs() < 1e-6);
    let none = group("GDRCars", 4, None);
    assert!((time_factor(&none, &road, day::WEEKDAY, 3.0) - 0.5).abs() < 1e-6);
}

/// At the ceiling every group keeps its share.
#[test]
fn ceiling_scales_groups_in_proportion() {
    let wanted = [90.0, 30.0, 0.3];
    let (t, capped) = group_targets(&wanted, DEFAULT_MAX_TRAFFIC);
    assert!(capped);
    assert!((t.iter().sum::<f64>() - DEFAULT_MAX_TRAFFIC as f64).abs() < 1e-9);
    assert!((t[0] / t[1] - 3.0).abs() < 1e-9);
}
