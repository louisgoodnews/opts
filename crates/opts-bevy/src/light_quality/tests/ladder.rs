//! The quality ladder table and its invariants.

use super::*;

#[test]
fn the_ladder_table_is_exact() {
    // (level, shafts, night, lanterns, headlights, cap, depth fade, mirrors, every, glass, bloom)
    #[rustfmt::skip]
    let table = [
        (0, Shafts::Medium, true, true, true, 64, true, 2, 2, true, false),
        (1, Shafts::Medium, true, true, true, 48, true, 2, 2, true, false),
        (2, Shafts::Medium, true, false, true, 16, false, 2, 2, true, false),
        (3, Shafts::Medium, true, false, false, 0, false, 2, 2, true, false),
        (4, Shafts::Low, false, false, false, 0, false, 1, 4, true, false),
        (5, Shafts::Off, false, false, false, 0, false, 1, 4, true, false),
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
        // Mirror pictures per frame never grow going down.
        assert!(
            u32::from(q.mirror_count) * u32::from(prev.mirror_every)
                <= u32::from(prev.mirror_count) * u32::from(q.mirror_every)
        );
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
