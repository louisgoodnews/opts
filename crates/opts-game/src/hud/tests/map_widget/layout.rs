//! Mini-map tests: the cluster rows, the profiles that place the map.

use super::*;

#[test]
fn the_cluster_has_its_rows_in_a_fixed_order_and_the_map_is_a_separate_slot() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let cluster = app
        .world_mut()
        .query::<(Entity, &build::HudSlot)>()
        .iter(app.world())
        .find(|(_, s)| s.0 == Slot::ClusterBottomLeft)
        .map(|(e, _)| e)
        .expect("cluster");
    let rows: Vec<widget::Row> = app
        .world()
        .get::<Children>(cluster)
        .expect("children")
        .iter()
        .filter_map(|c| app.world().get::<build::HudRow>(c).map(|r| r.0))
        .collect();
    assert_eq!(
        rows,
        [
            widget::Row::Hints,
            widget::Row::Alerts,
            widget::Row::Instruments
        ]
    );
    // The map is a sibling of the cluster, after it (bottom right).
    let parent = app
        .world()
        .get::<ChildOf>(cluster)
        .expect("parent")
        .parent();
    let siblings: Vec<Entity> = app
        .world()
        .get::<Children>(parent)
        .expect("c")
        .iter()
        .collect();
    let map_slot = app
        .world_mut()
        .query::<(Entity, &build::HudSlot)>()
        .iter(app.world())
        .find(|(_, s)| s.0 == Slot::MiniMap)
        .map(|(e, _)| e)
        .expect("map slot");
    let at = |e: Entity| siblings.iter().position(|s| *s == e).expect("in the row");
    assert!(at(cluster) < at(map_slot));
}

#[test]
fn every_shipped_profile_places_the_map_and_the_rows_in_the_cluster() {
    let profiles = HudProfiles::load(None);
    for kind in VehicleKind::ALL {
        let p = profiles.get(kind).expect("profile");
        assert_eq!(p.in_slot(Slot::MiniMap).len(), 1, "{}", kind.id());
        for row in widget::Row::ALL {
            assert!(!p.in_row(row).is_empty(), "{} {}", kind.id(), row.id());
        }
        assert_eq!(
            p.in_slot(Slot::ClusterBottomLeft).len(),
            widget::Row::ALL
                .iter()
                .map(|r| p.in_row(*r).len())
                .sum::<usize>()
        );
    }
}
