use super::*;

#[test]
fn the_ladder_table_is_exact() {
    // (level, shafts, night, lanterns, headlights, cap, depth fade, mirrors, every, glass, bloom)
    let table = [
        (
            0,
            Shafts::Medium,
            true,
            true,
            true,
            64,
            true,
            2,
            2,
            true,
            false,
        ),
        (
            1,
            Shafts::Medium,
            true,
            true,
            true,
            48,
            true,
            2,
            2,
            true,
            false,
        ),
        (
            2,
            Shafts::Medium,
            true,
            false,
            true,
            16,
            false,
            2,
            2,
            true,
            false,
        ),
        (
            3,
            Shafts::Medium,
            true,
            false,
            false,
            0,
            false,
            2,
            2,
            true,
            false,
        ),
        (
            4,
            Shafts::Low,
            false,
            false,
            false,
            0,
            false,
            0,
            2,
            true,
            false,
        ),
        (
            5,
            Shafts::Off,
            false,
            false,
            false,
            0,
            false,
            0,
            2,
            true,
            false,
        ),
    ];
    for (l, sh, night, lan, head, cap, fade, mc, me, glass, bloom) in table {
        let q = ladder_preset(l);
        assert_eq!(
            q,
            LightQuality {
                mode: LightMode::Automatic,
                level: l,
                shafts: sh,
                night_shafts: night,
                lantern_cones: lan,
                headlight_cones: head,
                cone_cap: cap,
                cone_depth_fade: fade,
                mirror_count: mc,
                mirror_every: me,
                glass_reflections: glass,
                bloom,
            },
            "level {l}"
        );
    }
    assert_eq!(ladder_preset(9), ladder_preset(5), "clamped");
    assert_eq!(LightQuality::default(), ladder_preset(0));
}

#[test]
fn the_ladder_never_adds_cost_going_down() {
    let mut prev = ladder_preset(0);
    for l in 1..=5 {
        let q = ladder_preset(l);
        assert!(q.shafts.steps() <= prev.shafts.steps());
        assert!(q.cone_cap <= prev.cone_cap);
        assert!(q.mirror_count <= prev.mirror_count);
        assert!(!q.lantern_cones || prev.lantern_cones);
        assert!(!q.headlight_cones || prev.headlight_cones);
        assert!(!q.night_shafts || prev.night_shafts);
        prev = q;
    }
}

#[test]
fn shafts_steps_and_names_round_trip() {
    let steps: Vec<u32> = Shafts::ALL.iter().map(|s| s.steps()).collect();
    assert_eq!(steps, [0, 8, 16, 32, 64]);
    for s in Shafts::ALL {
        assert_eq!(Shafts::parse(s.name()), Some(s));
    }
    assert_eq!(Shafts::parse("nope"), None);
    assert_eq!(LightMode::parse("custom"), Some(LightMode::Custom));
    assert_eq!(LightMode::parse("x"), None);
}

#[test]
fn clamping_keeps_values_in_range() {
    let q = LightQuality {
        cone_cap: 500,
        mirror_count: 9,
        mirror_every: 3,
        level: 40,
        ..LightQuality::default()
    }
    .clamped();
    assert_eq!(
        (q.cone_cap, q.mirror_count, q.mirror_every, q.level),
        (96, 2, 2, 5)
    );
}

/// 60 Hz deadline.
const T60: f32 = 1000.0 / 60.0;

/// The default thresholds without the start-up warm-up (tested on its own).
fn cfg() -> GovernorSettings {
    GovernorSettings {
        warmup_s: 0.0,
        ..GovernorSettings::default()
    }
}

/// Feeds `seconds` of frames given by `ms(i)`; returns (time, change) pairs.
fn feed_with(
    state: &mut GovernorState,
    cfg: &GovernorSettings,
    target: f32,
    seconds: f32,
    ms: impl Fn(usize) -> f32,
) -> Vec<(f32, Change)> {
    let mut t = 0.0;
    let mut i = 0;
    let mut out = Vec::new();
    while t < seconds {
        let m = ms(i);
        let dt = m / 1000.0;
        t += dt;
        i += 1;
        out.extend(step(state, cfg, target, m, dt).map(|c| (t, c)));
    }
    out
}

/// Frames of `ms` each for `seconds`, judged against `target`.
fn feed_at(
    state: &mut GovernorState,
    cfg: &GovernorSettings,
    target: f32,
    ms: f32,
    seconds: f32,
) -> Vec<Change> {
    feed_with(state, cfg, target, seconds, |_| ms)
        .into_iter()
        .map(|(_, c)| c)
        .collect()
}

#[test]
fn vsync_at_sixty_hertz_never_steps_down() {
    let mut s = GovernorState::default();
    // Every frame 16.7 ms, a little jitter, 5 minutes: not a single late frame.
    let changes = feed_with(&mut s, &cfg(), T60, 300.0, |i| 16.7 + (i % 3) as f32 * 0.4);
    assert!(changes.is_empty(), "{changes:?}");
    assert_eq!(s.level, 0);
    assert_eq!(s.late_share, 0.0);
}

#[test]
fn a_quarter_of_the_frames_a_vblank_late_steps_down_once_per_five_seconds_to_five() {
    let mut s = GovernorState::default();
    let changes = feed_with(&mut s, &cfg(), T60, 60.0, |i| {
        if i % 4 == 3 {
            33.3
        } else {
            16.7
        }
    });
    assert_eq!(changes.len(), 5, "{changes:?}");
    assert!(changes.iter().all(|(_, c)| c.to == c.from + 1));
    for w in changes.windows(2) {
        assert!(w[1].0 - w[0].0 >= 5.0 - 0.05, "{changes:?}");
    }
    assert!((changes[0].1.late_share - 0.25).abs() < 0.02);
    assert!((changes[0].1.target_ms - T60).abs() < 1e-3);
    assert_eq!(s.level, 5);
    // And it stays there.
    let more = feed_with(&mut s, &cfg(), T60, 60.0, |i| {
        if i % 4 == 3 {
            33.3
        } else {
            16.7
        }
    });
    assert!(more.is_empty());
}

#[test]
fn nine_percent_late_is_below_the_threshold() {
    let mut s = GovernorState::default();
    // 11 of 120 frames late = 9.2 %.
    let changes = feed_with(&mut s, &cfg(), T60, 60.0, |i| {
        if i % 120 < 11 {
            33.3
        } else {
            16.7
        }
    });
    assert!(changes.is_empty(), "{changes:?}");
}

#[test]
fn it_needs_a_full_window_and_two_seconds() {
    let mut s = GovernorState::default();
    // All frames late (40 ms): the 120-frame window is full after 4.8 s, plus 2 s.
    let changes = feed_with(&mut s, &cfg(), T60, 10.0, |_| 40.0);
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert!(changes[0].0 > 6.5 && changes[0].0 < 7.2, "{changes:?}");
}

#[test]
fn recovery_steps_up_after_twenty_clean_seconds_not_before() {
    let mut s = GovernorState::at(3);
    // The window fills first (2 s), then 20 s clean.
    assert!(
        feed_at(&mut s, &cfg(), T60, 16.7, 21.0).is_empty(),
        "not yet"
    );
    assert_eq!(s.level, 3);
    let c = feed_at(&mut s, &cfg(), T60, 16.7, 3.0);
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].from, c[0].to), (3, 2));
    assert_eq!(c[0].late_share, 0.0);
}

#[test]
fn a_single_late_frame_restarts_the_clean_time() {
    let mut s = GovernorState::at(3);
    let c = feed_with(&mut s, &cfg(), T60, 40.0, |i| {
        if i % 1000 == 999 {
            33.3
        } else {
            16.7
        }
    });
    // About 1000 frames (16.7 s) per late frame: a clean 20 s never completes.
    assert!(c.is_empty(), "{c:?}");
}

/// Feeds frames of `ms` until the governor changes level (or `max_s` pass); returns the change.
fn until_change(state: &mut GovernorState, cfg: &GovernorSettings, ms: f32, max_s: f32) -> Change {
    let dt = ms / 1000.0;
    for _ in 0..(max_s / dt) as usize {
        if let Some(c) = step(state, cfg, T60, ms, dt) {
            return c;
        }
    }
    panic!(
        "no change within {max_s} s at {ms} ms (level {})",
        state.level
    );
}

#[test]
fn a_failed_probation_doubles_the_wait_up_to_160_seconds() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    assert_eq!(s.up_wait_s, 20.0);
    let mut waits = Vec::new();
    for _ in 0..6 {
        // Clean until the governor tries a level up ...
        let up = until_change(&mut s, &cfg, 16.7, 400.0);
        assert_eq!((up.from, up.to), (1, 0));
        // ... and the lower level is too much: down again within the probation.
        let down = until_change(&mut s, &cfg, 40.0, 29.0);
        assert_eq!((down.from, down.to), (0, 1));
        waits.push(s.up_wait_s);
    }
    assert_eq!(waits, [40.0, 80.0, 160.0, 160.0, 160.0, 160.0]);
}

#[test]
fn the_wait_is_really_the_doubled_one() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    until_change(&mut s, &cfg, 16.7, 400.0);
    until_change(&mut s, &cfg, 40.0, 29.0);
    assert_eq!(s.up_wait_s, 40.0);
    // Clean time needed: the 40 s wait, not 20 (the window refill is part of the clean time).
    assert!(feed_at(&mut s, &cfg, T60, 16.7, 38.0).is_empty());
    assert_eq!(feed_at(&mut s, &cfg, T60, 16.7, 4.0).len(), 1);
}

#[test]
fn the_backoff_resets_after_five_minutes_without_a_step_down() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    until_change(&mut s, &cfg, 16.7, 400.0); // up
    until_change(&mut s, &cfg, 40.0, 29.0); // failed probation
    assert_eq!(s.up_wait_s, 40.0);
    // 5 minutes without a down-step (level 1 goes up again after 40 s meanwhile).
    feed_with(&mut s, &cfg, T60, 305.0, |_| 16.7);
    assert_eq!(s.up_wait_s, 20.0);
}

#[test]
fn a_down_step_long_after_an_up_step_is_no_failed_probation() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    until_change(&mut s, &cfg, 16.7, 400.0); // up to 0
    feed_with(&mut s, &cfg, T60, 60.0, |_| 16.7);
    let down = until_change(&mut s, &cfg, 40.0, 29.0);
    assert_eq!(down.to, 1);
    assert_eq!(s.up_wait_s, 20.0);
}

#[test]
fn vsync_off_at_144_hz_with_a_100_fps_target() {
    let target = target_ms(false, Some(144_000), 100);
    assert!((target - 10.0).abs() < 1e-4);
    // 7 ms frames (143 fps) are fine.
    let mut s = GovernorState::default();
    assert!(feed_at(&mut s, &cfg(), target, 7.0, 60.0).is_empty());
    // 14 ms frames (71 fps) are late (> 12.5 ms): steps down.
    let c = feed_at(&mut s, &cfg(), target, 14.0, 12.0);
    assert!(!c.is_empty());
    assert!((c[0].target_ms - 10.0).abs() < 1e-4);
    // 12 ms frames are not late.
    let mut s = GovernorState::default();
    assert!(feed_at(&mut s, &cfg(), target, 12.0, 60.0).is_empty());
}

#[test]
fn a_thirty_fps_target() {
    let target = target_ms(false, None, 30);
    assert!((target - 33.333).abs() < 0.01);
    let mut s = GovernorState::default();
    assert!(feed_at(&mut s, &cfg(), target, 33.3, 60.0).is_empty());
    assert!(!feed_at(&mut s, &cfg(), target, 45.0, 20.0).is_empty());
}

#[test]
fn the_budget_follows_vsync_the_monitor_and_the_requested_fps() {
    // vsync on: the monitor's interval; unknown refresh: 60 Hz.
    assert!((target_ms(true, Some(59_940), 60) - 16.683).abs() < 0.01);
    assert!((target_ms(true, None, 60) - 16.667).abs() < 0.01);
    // vsync on, 144 Hz, default target 60: the lower target wins.
    assert!((target_ms(true, Some(144_000), 60) - 16.667).abs() < 0.01);
    // vsync on, 30 Hz panel, target 60: the interval is longer than the wish.
    assert!((target_ms(true, Some(30_000), 60) - 33.333).abs() < 0.01);
    // vsync on, 144 Hz, target 240: not faster than the display.
    assert!((target_ms(true, Some(144_000), 240) - 6.944).abs() < 0.01);
    // vsync off ignores the monitor; the fps is clamped to 30..=240.
    assert!((target_ms(false, Some(144_000), 60) - 16.667).abs() < 0.01);
    assert!((target_ms(false, None, 5) - 33.333).abs() < 0.01);
    assert!((target_ms(false, None, 1000) - 4.1667).abs() < 0.01);
    assert!(is_vsync(bevy::window::PresentMode::Fifo));
    assert!(is_vsync(bevy::window::PresentMode::AutoVsync));
    assert!(!is_vsync(bevy::window::PresentMode::AutoNoVsync));
    assert!(!is_vsync(bevy::window::PresentMode::Immediate));
}

#[test]
fn level_zero_never_goes_up_and_five_never_down() {
    let mut s = GovernorState::default();
    assert!(feed_at(&mut s, &cfg(), T60, 5.0, 60.0).is_empty());
    assert_eq!(s.level, 0);
    let mut s = GovernorState::at(5);
    assert!(feed_at(&mut s, &cfg(), T60, 40.0, 60.0).is_empty());
    assert_eq!(s.level, 5);
}

#[test]
fn isolated_hitches_are_not_counted() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    // Clean frames with a 120 ms hitch every second: the hitch is ignored, level 1 recovers.
    let mut changes = Vec::new();
    for _ in 0..40 {
        for i in 0..60 {
            let ms = if i == 30 { 120.0 } else { 16.7 };
            changes.extend(step(&mut s, &cfg, T60, ms, ms / 1000.0));
        }
    }
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0].to, 0);
    // A hitch frame never enters the window.
    let mut t = GovernorState::default();
    step(&mut t, &cfg, T60, 200.0, 0.2);
    assert_eq!(t.window_len(), 0);
}

#[test]
fn a_sustained_slowdown_is_not_mistaken_for_hitches() {
    let mut s = GovernorState::default();
    let changes = feed_at(&mut s, &cfg(), T60, 80.0, 30.0);
    assert!(
        !changes.is_empty(),
        "80 ms frames for 30 s must lower the level"
    );
}

#[test]
fn the_first_seconds_after_the_gate_opens_are_ignored() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::default();
    // 4.9 s of terrible frames do not even enter the window.
    let c = feed_at(&mut s, &cfg, T60, 90.0, 4.9);
    assert!(c.is_empty());
    assert_eq!(s.window_len(), 0);
    // After a pause the warm-up applies again, the level stays.
    let mut s = GovernorState::at(2);
    feed_at(&mut s, &cfg, T60, 16.7, 6.0);
    assert!(s.window_len() > 0);
    s.pause();
    assert_eq!((s.level, s.window_len()), (2, 0));
    feed_at(&mut s, &cfg, T60, 16.7, 4.0);
    assert_eq!(s.window_len(), 0);
}

#[test]
fn a_step_restarts_the_window() {
    let cfg = cfg();
    let mut s = GovernorState::default();
    let c = feed_at(&mut s, &cfg, T60, 40.0, 7.5);
    assert_eq!(c.len(), 1);
    assert!(
        s.window_len() < cfg.window_frames,
        "the window starts again after a step"
    );
}

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
