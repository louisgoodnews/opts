//! The cabin-layer state machine: hysteresis, and the camera's layer set.

use super::*;

#[test]
fn the_cabin_layer_has_hysteresis_and_an_interior_view_always_wants_it() {
    // Outside, entering at 12 m, leaving at 15 m.
    assert!(!cabin_layer_wanted(false, false, Some(13.0)));
    assert!(cabin_layer_wanted(false, false, Some(12.0)));
    assert!(cabin_layer_wanted(false, false, Some(3.0)));
    assert!(
        cabin_layer_wanted(true, false, Some(14.0)),
        "stays between 12 and 15"
    );
    assert!(cabin_layer_wanted(true, false, Some(15.0)));
    assert!(!cabin_layer_wanted(true, false, Some(15.1)));
    assert!(
        !cabin_layer_wanted(false, false, Some(14.0)),
        "does not join between 12 and 15"
    );
    // An interior view wants it at any distance (a camera glued into the cockpit).
    assert!(cabin_layer_wanted(false, true, Some(40.0)));
    // No player vehicle: never.
    assert!(!cabin_layer_wanted(true, true, None));
    assert!(!cabin_layer_wanted(false, false, None));
}

#[test]
fn a_walk_away_and_back_switches_once_each_way() {
    let mut on = false;
    let mut switches = 0;
    // 30 m away, in to 5 m, out to 30 m, in steps of 0.5 m with a wobble of +-0.4 m.
    let path = (0..120)
        .map(|i| 30.0 - i as f32 * 0.5)
        .chain((0..120).map(|i| 5.0 + i as f32 * 0.5));
    for (i, d) in path.enumerate() {
        let wobble = if i % 2 == 0 { 0.4 } else { -0.4 };
        let next = cabin_layer_wanted(on, false, Some(d + wobble));
        if next != on {
            switches += 1;
        }
        on = next;
    }
    assert_eq!(switches, 2);
}

#[test]
fn the_cabin_camera_layers_are_world_plus_cabin() {
    let outside = camera_layers(false);
    let inside = camera_layers(true);
    let world = RenderLayers::layer(WORLD_LAYER);
    let cabin = RenderLayers::layer(CABIN_LAYER);
    // An entity without the component is on layer 0 and must stay visible either way.
    assert!(outside.intersects(&RenderLayers::default()));
    assert!(inside.intersects(&RenderLayers::default()));
    assert!(!outside.intersects(&cabin));
    assert!(inside.intersects(&cabin));
    assert!(inside.intersects(&world) && outside.intersects(&world));
}
