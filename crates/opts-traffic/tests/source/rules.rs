//! Traffic rules: which path an owner's rule addresses and how the values are read.

use super::*;

#[test]
fn rules_apply_to_the_addressed_path_of_their_owner() {
    let rule = |owner, path_index, rule_type: &str, value| MapTrafficRule {
        owner: Some(owner),
        path_index,
        rule_type: rule_type.into(),
        value,
        flag: u32::from(rule_type == "trafficdensity" && value > 1.0) * 4,
    };
    let me = RuleOwner::Spline {
        index: 0,
        id_code: 7,
    };
    let other = RuleOwner::Spline {
        index: 1,
        id_code: 8,
    };
    let tile = MapTile {
        splines: vec![spline(0.0, 100.0)],
        rules: vec![
            rule(me, 1, "speedlimit", 36.0),
            rule(me, 1, "trafficdensity", 0.5),
            rule(me, 1, "trafficdensity", 3.0),
            rule(me, 1, "priority", 192.0),
            rule(me, 1, "trucks", 0.0),
            rule(me, 0, "overtaking_prohib", 0.0),
            rule(other, 0, "speedlimit", 5.0),
            rule(me, 0, "speedlimit", 0.0),
            rule(me, 0, "speedlimit", 250.0),
        ],
        ..MapTile::default()
    };
    let mut src = Fixed {
        sli: vec![spline_path(1.75, 0), spline_path(-1.75, 0)],
        sco: Vec::new(),
    };
    let segs = segments_from_tile(&tile, DVec2::ZERO, &mut src);
    // Path 0: only the overtaking ban; limits of 0 and 250 mean none, the other owner's is
    // ignored.
    assert_eq!(segs[0].speed_limit, None);
    assert!(segs[0].rules.no_overtaking && segs[0].rules.density.is_empty());
    // Path 1: 36 km/h = 10 m/s, density 0.5 for group 0 and 3 (unclamped) for group 4 (the
    // rule's flag), priority 192, trucks.
    assert!((segs[1].speed_limit.unwrap() - 10.0).abs() < 1e-9);
    assert_eq!(
        (
            segs[1].rules.density.get(0),
            segs[1].rules.density.get(4),
            segs[1].rules.priority
        ),
        (Some(0.5), Some(3.0), Some(192))
    );
    assert_eq!(segs[1].rules.density.get(1), None);
    assert!(segs[1].rules.trucks && !segs[1].rules.bus && !segs[1].rules.no_overtaking);
}

#[test]
fn rule_indices_count_every_path_even_filtered_ones() {
    // Path 0 is a pedestrian path; the rule addresses path 1, the road lane.
    let mut ped = spline_path(3.0, 0);
    ped.path_type = 1;
    let owner = RuleOwner::Spline {
        index: 0,
        id_code: 7,
    };
    let tile = MapTile {
        splines: vec![spline(0.0, 50.0)],
        rules: vec![MapTrafficRule {
            owner: Some(owner),
            path_index: 1,
            rule_type: "speedlimit".into(),
            value: 72.0,
            flag: 0,
        }],
        ..MapTile::default()
    };
    let mut src = Fixed {
        sli: vec![ped, spline_path(0.0, 0)],
        sco: Vec::new(),
    };
    let segs = segments_from_tile(&tile, DVec2::ZERO, &mut src);
    assert_eq!(segs.len(), 2);
    assert_eq!(segs[0].speed_limit, None);
    assert!((segs[1].speed_limit.unwrap() - 20.0).abs() < 1e-9);
}
