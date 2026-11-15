//! The bridge without the install: the start state, the trigger, and the release of a held key.

use bevy::prelude::*;

use super::{app, lights, press};

#[test]
fn lights_start_dark_and_the_door_closed() {
    let (mut app, _, door) = app();
    app.update();
    let lights = lights(&mut app);
    assert_eq!(lights.len(), 3);
    assert!(lights.iter().all(|(i, v)| *i == 0.0 && !v), "{lights:?}");
    let tf = app.world().get::<Transform>(door).unwrap();
    assert!(tf.translation.length() < 1e-6);
}

#[test]
fn a_key_raises_the_trigger_that_opens_the_door_and_switches_lights() {
    let (mut app, _, door) = app();
    app.update();
    press(&mut app, KeyCode::NumpadDivide);

    // `anim_trans door_0 0.8` along the frame's X axis (OMSI x = Bevy x).
    let tf = *app.world().get::<Transform>(door).unwrap();
    assert!(
        (tf.translation - Vec3::new(0.8, 0.0, 0.0)).length() < 1e-5,
        "{tf:?}"
    );

    let lights = lights(&mut app);
    // Interior light on at factor 1; of the spotlights only number 1 (Spot_Select = 1).
    assert!(lights[0].0 > 0.0 && lights[0].1, "{lights:?}");
    assert!(lights[1].0 == 0.0 && !lights[1].1, "{lights:?}");
    assert!(lights[2].0 > 0.0 && lights[2].1, "{lights:?}");
}

#[test]
fn releasing_a_held_key_raises_its_off_trigger() {
    let (mut app, _, door) = app();
    app.update();
    press(&mut app, KeyCode::NumpadDivide);
    // `horn` is a held binding: its release raises `horn_off`, which closes the door here.
    press(&mut app, KeyCode::KeyH);
    app.update();
    let tf = app.world().get::<Transform>(door).unwrap();
    assert!(tf.translation.length() < 1e-5, "{tf:?}");
}
