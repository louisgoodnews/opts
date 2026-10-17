use super::*;

/// Feeds `seconds` of frames of `ms` each; returns the changes.
fn feed(state: &mut GovernorState, cfg: &GovernorSettings, ms: f32, seconds: f32) -> Vec<Change> {
    let dt = ms / 1000.0;
    let n = (seconds / dt).round() as usize;
    (0..n).filter_map(|_| step(state, cfg, ms, dt)).collect()
}

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

#[test]
fn slow_frames_step_down_after_two_seconds_but_not_before() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::default();
    // The judgement needs the full window (120 frames = 2.2 s at 18 ms), then 2 s over 15 ms.
    let none = feed(&mut s, &cfg, 18.0, 4.0);
    assert!(none.is_empty(), "{none:?}");
    let c = feed(&mut s, &cfg, 18.0, 1.0);
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].from, c[0].to), (0, 1));
    assert!(c[0].p95_ms > 15.0);
    assert_eq!(s.level, 1);
}

#[test]
fn hysteresis_between_eleven_and_fifteen_ms_changes_nothing() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::at(2);
    assert!(feed(&mut s, &cfg, 13.0, 60.0).is_empty());
    assert_eq!(s.level, 2);
}

#[test]
fn at_most_one_step_per_five_seconds() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::default();
    let changes = feed(&mut s, &cfg, 30.0, 40.0);
    // 40 s of very slow frames: steps are >= 5 s apart, so at most 8 and never past level 5.
    assert!(changes.len() <= 8, "{changes:?}");
    assert_eq!(s.level, 5);
    assert_eq!(changes.len(), 5);
    // The first step needs the window (4 s of 30 ms frames) then each next one 5 s.
    let mut t = GovernorState::default();
    let mut times = Vec::new();
    let dt = 0.030;
    for i in 0..2000 {
        if step(&mut t, &cfg, 30.0, dt).is_some() {
            times.push(i as f32 * dt);
        }
    }
    for w in times.windows(2) {
        assert!(w[1] - w[0] >= 5.0 - dt, "{times:?}");
    }
}

#[test]
fn recovery_steps_up_after_fifteen_seconds_below_eleven_ms() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::at(3);
    assert!(feed(&mut s, &cfg, 8.0, 14.0).is_empty(), "not yet");
    assert_eq!(s.level, 3);
    let c = feed(&mut s, &cfg, 8.0, 3.0);
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].from, c[0].to), (3, 2));
}

#[test]
fn level_zero_never_goes_up_and_five_never_down() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::default();
    assert!(feed(&mut s, &cfg, 5.0, 60.0).is_empty());
    assert_eq!(s.level, 0);
    let mut s = GovernorState::at(5);
    assert!(feed(&mut s, &cfg, 40.0, 60.0).is_empty());
    assert_eq!(s.level, 5);
}

#[test]
fn isolated_hitches_are_not_counted() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::at(1);
    // Fast frames with a 120 ms hitch every second: the p95 stays low, level 1 recovers.
    let mut changes = Vec::new();
    for sec in 0..30 {
        for i in 0..100 {
            let ms = if i == 50 { 120.0 } else { 8.0 };
            changes.extend(step(&mut s, &cfg, ms, ms / 1000.0));
        }
        let _ = sec;
    }
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0].to, 0);
    // A hitch frame never enters the window.
    let mut t = GovernorState::default();
    step(&mut t, &cfg, 200.0, 0.2);
    assert_eq!(t.window_len(), 0);
}

#[test]
fn a_sustained_slowdown_is_not_mistaken_for_hitches() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::default();
    let changes = feed(&mut s, &cfg, 80.0, 30.0);
    assert!(
        !changes.is_empty(),
        "80 ms frames for 30 s must lower the level"
    );
}

#[test]
fn a_step_restarts_the_window() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::default();
    let c = feed(&mut s, &cfg, 25.0, 5.5);
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
