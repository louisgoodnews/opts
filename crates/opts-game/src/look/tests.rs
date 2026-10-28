//! Unit tests for `super` (`look.rs`).

use super::*;

fn reinhard(x: f32) -> f32 {
    x / (1.0 + x)
}

#[test]
fn vivid_is_the_default_and_keys_round_trip() {
    assert_eq!(Look::default(), Look::Vivid);
    for look in Look::ALL {
        assert_eq!(Look::parse(look.key()), Some(look));
    }
    assert_eq!(Look::parse(" Bevy_Default "), Some(Look::BevyDefault));
    assert_eq!(Look::parse("sepia"), None);
}

#[test]
fn bevy_default_is_exactly_todays_behaviour() {
    let p = Look::BevyDefault.params();
    assert_eq!(p.tonemapping, Tonemapping::default());
    assert_eq!(grading_for(p), ColorGrading::default());
    assert_eq!(p.ambient_scale, 1.0);
    assert_eq!(Look::Natural.params(), p, "until the calibration of step 2");
}

#[test]
fn vivid_uses_the_provisional_values() {
    let p = Look::Vivid.params();
    assert_eq!(p.tonemapping, Tonemapping::SomewhatBoringDisplayTransform);
    assert_eq!(p.ambient_scale, 0.5);
    let g = grading_for(p);
    assert!((g.global.post_saturation - 1.1).abs() < 1e-6);
}

#[test]
fn ambient_scale_fades_out_at_dusk_and_night() {
    assert_eq!(effective_ambient_scale(0.5, 60.0), 0.5);
    assert_eq!(effective_ambient_scale(0.5, 25.0), 0.5);
    assert_eq!(effective_ambient_scale(0.5, 0.0), 1.0);
    assert_eq!(effective_ambient_scale(0.5, -30.0), 1.0);
    let dusk = effective_ambient_scale(0.5, 8.0);
    assert!(dusk > 0.5 && dusk < 1.0);
    assert_eq!(
        effective_ambient_scale(1.0, 40.0),
        1.0,
        "Bevy default never changes it"
    );
}

#[test]
fn the_hdr_clear_colour_round_trips_through_the_tonemapper() {
    for display in [
        [0.5, 0.6, 0.8],
        [0.30, 0.35, 0.45],
        [0.8, 0.8, 0.8],
        [0.0, 0.0, 0.0],
    ] {
        let x = hdr_clear_color(Tonemapping::SomewhatBoringDisplayTransform, display);
        let back = somewhat_boring(x);
        for i in 0..3 {
            assert!(
                (back[i] - display[i]).abs() < 0.01,
                "{display:?} -> {x:?} -> {back:?}"
            );
        }
        let r = hdr_clear_color(Tonemapping::Reinhard, display);
        for i in 0..3 {
            assert!((reinhard(r[i]) - display[i]).abs() < 0.01);
        }
    }
    assert_eq!(
        hdr_clear_color(Tonemapping::None, [0.2, 0.4, 0.6]),
        [0.2, 0.4, 0.6]
    );
}

fn app_with_look(look: Look) -> App {
    let mut app = App::new();
    app.insert_resource(ActiveLook(look));
    app.add_plugins(LookPlugin);
    app
}

#[test]
fn cameras_follow_the_active_look_live() {
    let mut app = app_with_look(Look::Vivid);
    let cam = app
        .world_mut()
        .spawn((Camera3d::default(), opts_bevy::MainCamera))
        .id();
    app.update();
    let tm = *app.world().get::<Tonemapping>(cam).expect("tonemapping");
    assert_eq!(tm, Tonemapping::SomewhatBoringDisplayTransform);
    let g = app.world().get::<ColorGrading>(cam).expect("grading");
    assert!((g.global.post_saturation - 1.1).abs() < 1e-6);

    app.world_mut().resource_mut::<ActiveLook>().0 = Look::BevyDefault;
    app.update();
    assert_eq!(
        *app.world().get::<Tonemapping>(cam).expect("tonemapping"),
        Tonemapping::default()
    );
    assert_eq!(
        *app.world().get::<ColorGrading>(cam).expect("grading"),
        ColorGrading::default()
    );
    // A camera spawned later gets the current look although the look did not change.
    let late = app
        .world_mut()
        .spawn((Camera3d::default(), opts_bevy::MainCamera))
        .id();
    app.update();
    assert_eq!(
        *app.world().get::<Tonemapping>(late).expect("tonemapping"),
        Tonemapping::default()
    );
}
