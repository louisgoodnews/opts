//! Unit tests of `spatial_zones`: the zone box and the message registration. The tracker and
//! the sampling gate are tested in `opts-spatial-zones-core`.

use bevy::math::Vec2;
use bevy::prelude::*;

use super::*;

fn tunnel() -> SpatialZone {
    SpatialZone {
        id: ZoneId(1),
        kind: ZoneKind::Tunnel,
        min: Vec2::new(0.0, 0.0),
        max: Vec2::new(100.0, 10.0),
    }
}

#[test]
fn zone_contains_its_box_and_edges_only() {
    let z = tunnel();
    assert!(z.contains(Vec2::new(50.0, 5.0)));
    assert!(z.contains(Vec2::new(0.0, 10.0)));
    assert!(!z.contains(Vec2::new(-0.1, 5.0)));
    assert!(!z.contains(Vec2::new(50.0, 10.5)));
}

#[test]
fn plugin_registers_zone_messages() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SpatialZonesPlugin));
    let z = tunnel();
    app.world_mut().write_message(ZoneEntered {
        zone: z.id,
        kind: z.kind,
    });
    app.world_mut().write_message(ZoneLeft {
        zone: z.id,
        kind: z.kind,
    });
    let entered = app.world().resource::<Messages<ZoneEntered>>();
    assert_eq!(entered.len(), 1);
    assert!(app.world().contains_resource::<Messages<ZoneLeft>>());
}
