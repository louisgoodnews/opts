//! Unit tests for `super` (`motion.rs`).

use super::*;

#[test]
fn durations_match_the_tokens() {
    assert_eq!(MotionRole::HoverFocus.token().millis(), 100);
    assert_eq!(MotionRole::Panel.token().millis(), 250);
    assert_eq!(MotionRole::Screen.token().millis(), 400);
}

#[test]
fn reduced_motion_zeroes_every_duration() {
    let m = MotionTokens {
        reduced_motion: true,
    };
    for r in [
        MotionRole::HoverFocus,
        MotionRole::Panel,
        MotionRole::Screen,
    ] {
        assert_eq!(m.duration_for(r), Duration::ZERO);
    }
    assert_eq!(
        MotionTokens::default().duration_for(MotionRole::Panel),
        Duration::from_millis(250)
    );
}

#[test]
fn either_source_reduces_motion() {
    assert!(!MotionPreference::default().reduced());
    assert!(MotionPreference {
        user_setting: true,
        os_hint: false
    }
    .reduced());
    assert!(MotionPreference {
        user_setting: false,
        os_hint: true
    }
    .reduced());
}

#[test]
fn easing_is_monotonic_and_hits_the_ends() {
    for e in [
        EasingCurve::Standard,
        EasingCurve::Emphasized,
        EasingCurve::Decelerated,
        EasingCurve::Accelerated,
    ] {
        assert!(e.apply(0.0).abs() < 1e-3);
        assert!((e.apply(1.0) - 1.0).abs() < 1e-3);
        let mut prev = 0.0;
        for i in 1..=20 {
            let v = e.apply(i as f32 / 20.0);
            assert!(v + 1e-4 >= prev, "{e:?} not monotonic");
            prev = v;
        }
    }
}

fn app(reduced: bool) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).insert_resource(
        bevy::time::TimeUpdateStrategy::ManualDuration(Duration::ZERO),
    );
    let mut theme = MaterialTheme::default();
    theme.motion.reduced_motion = reduced;
    app.insert_resource(theme)
        .add_systems(Update, drive_background_transitions);
    app
}

#[test]
fn a_transition_ends_on_the_target_colour_and_removes_itself() {
    let mut app = app(false);
    let e = app
        .world_mut()
        .spawn((
            BackgroundColor(Color::BLACK),
            BackgroundTransition::new(Color::BLACK, Color::WHITE, MotionRole::HoverFocus),
        ))
        .id();
    app.update();
    // advance real time by more than 100 ms
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(150),
    ));
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(e).map(|b| b.0),
        Some(Color::WHITE)
    );
    assert!(app.world().get::<BackgroundTransition>(e).is_none());
}

#[test]
fn reduced_motion_jumps_straight_to_the_target() {
    let mut app = app(true);
    let e = app
        .world_mut()
        .spawn((
            BackgroundColor(Color::BLACK),
            BackgroundTransition::new(Color::BLACK, Color::WHITE, MotionRole::Panel),
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(e).map(|b| b.0),
        Some(Color::WHITE)
    );
    assert!(app.world().get::<BackgroundTransition>(e).is_none());
}
