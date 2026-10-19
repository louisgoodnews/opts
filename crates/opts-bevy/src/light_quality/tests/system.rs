//! The resolved quality and the governor system.

use super::*;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<Time<Real>>()
        .add_plugins(LightQualityPlugin);
    app
}

#[test]
fn the_resolved_quality_follows_mode_level_and_custom_values() {
    let mut app = app();
    app.update();
    assert_eq!(*app.world().resource::<LightQuality>(), ladder_preset(0));
    app.world_mut().resource_mut::<GovernorState>().level = 3;
    app.update();
    assert_eq!(app.world().resource::<LightQuality>().level, 3);
    assert_eq!(app.world().resource::<LightQuality>().cone_cap, 0);

    // Automatic to custom: the level stays, the values are the user's.
    let mut custom = ladder_preset(3);
    custom.cone_cap = 80;
    custom.bloom = true;
    *app.world_mut().resource_mut::<LightQualityInput>() = LightQualityInput {
        mode: LightMode::Custom,
        custom,
    };
    app.update();
    let q = *app.world().resource::<LightQuality>();
    assert_eq!(
        (q.mode, q.level, q.cone_cap, q.bloom),
        (LightMode::Custom, 3, 80, true)
    );

    // Custom to automatic restarts at level 0.
    app.world_mut().resource_mut::<LightQualityInput>().mode = LightMode::Automatic;
    app.update();
    let q = *app.world().resource::<LightQuality>();
    assert_eq!((q.mode, q.level), (LightMode::Automatic, 0));
    assert_eq!(q, ladder_preset(0));
}

#[test]
fn the_governor_system_obeys_gate_and_mode() {
    let mut app = app();
    app.insert_resource(GovernorSettings {
        window_frames: 4,
        down_after_s: 0.0,
        min_step_interval_s: 0.0,
        warmup_s: 0.0,
        ..GovernorSettings::default()
    });
    app.update();
    let slow = |app: &mut App| {
        let mut t = app.world_mut().resource_mut::<Time<Real>>();
        t.advance_by(std::time::Duration::from_millis(30));
    };
    // Gate closed: nothing counts.
    for _ in 0..20 {
        slow(&mut app);
        app.update();
    }
    assert_eq!(app.world().resource::<GovernorState>().level, 0);
    // Gate open: it steps down.
    app.world_mut().resource_mut::<GovernorGate>().open = true;
    for _ in 0..20 {
        slow(&mut app);
        app.update();
    }
    assert!(app.world().resource::<GovernorState>().level >= 1);
    // Custom: frozen.
    let lvl = app.world().resource::<GovernorState>().level;
    app.world_mut().resource_mut::<LightQualityInput>().mode = LightMode::Custom;
    for _ in 0..30 {
        slow(&mut app);
        app.update();
    }
    assert_eq!(app.world().resource::<GovernorState>().level, lvl);
}

#[test]
fn the_budget_reads_the_window_and_its_monitor_and_an_unfocused_window_pauses() {
    use bevy::window::{Monitor, OnMonitor, PresentMode, PrimaryWindow};
    let mut app = app();
    let monitor = app
        .world_mut()
        .spawn(Monitor {
            name: None,
            physical_height: 1080,
            physical_width: 1920,
            physical_position: IVec2::ZERO,
            refresh_rate_millihertz: Some(144_000),
            scale_factor: 1.0,
            video_modes: Vec::new(),
        })
        .id();
    let window = app
        .world_mut()
        .spawn((
            Window {
                present_mode: PresentMode::AutoVsync,
                ..default()
            },
            PrimaryWindow,
            OnMonitor(monitor),
        ))
        .id();
    app.world_mut().insert_resource(GovernorTargetFps(240));
    app.update();
    let b = *app.world().resource::<GovernorBudget>();
    assert!(b.vsync && b.window_active);
    assert_eq!(b.refresh_millihertz, Some(144_000));
    assert!((b.target_ms - 6.944).abs() < 0.01);
    // Vsync off: the requested fps.
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .present_mode = PresentMode::AutoNoVsync;
    app.world_mut().insert_resource(GovernorTargetFps(100));
    app.update();
    let b = *app.world().resource::<GovernorBudget>();
    assert!(!b.vsync);
    assert!((b.target_ms - 10.0).abs() < 1e-3);
    // Unfocused: the governor counts nothing.
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    app.world_mut().resource_mut::<GovernorGate>().open = true;
    app.insert_resource(GovernorSettings {
        window_frames: 4,
        down_after_s: 0.0,
        min_step_interval_s: 0.0,
        warmup_s: 0.0,
        ..GovernorSettings::default()
    });
    for _ in 0..20 {
        let mut t = app.world_mut().resource_mut::<Time<Real>>();
        t.advance_by(std::time::Duration::from_millis(40));
        app.update();
    }
    assert!(!app.world().resource::<GovernorBudget>().window_active);
    assert_eq!(app.world().resource::<GovernorState>().level, 0);
}
