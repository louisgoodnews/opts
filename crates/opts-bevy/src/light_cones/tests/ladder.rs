//! The level ladder: which cones show at each light quality level, and the live changes.

use super::*;

/// Ladder levels 0 to 5 show the expected cones: both kinds on levels 0 and 1, headlights only
/// on level 2, nothing from level 3 (cap 0 or both switches off).
#[test]
pub(super) fn the_ladder_levels_show_the_expected_cones() {
    let expected = [
        (0, 3, 3),
        (1, 3, 3),
        (2, 0, 3),
        (3, 0, 0),
        (4, 0, 0),
        (5, 0, 0),
    ];
    for (level, lanterns, heads) in expected {
        let mut app = app(level);
        let (l, h) = spawn_mixed(&mut app);
        run(&mut app, 80);
        assert_eq!(visible(&app, &l), lanterns, "level {level} lanterns");
        assert_eq!(visible(&app, &h), heads, "level {level} headlights");
    }
}

/// A live change of the quality hides (or shows) cones at once, not after the next interval.
#[test]
pub(super) fn a_live_ladder_change_applies_in_the_next_frame() {
    let mut app = app(0);
    let (l, h) = spawn_mixed(&mut app);
    run(&mut app, 80);
    assert_eq!(visible(&app, &l) + visible(&app, &h), 6);
    app.insert_resource(custom(ladder_preset(2)));
    run(&mut app, 1);
    assert_eq!((visible(&app, &l), visible(&app, &h)), (0, 3));
    app.insert_resource(custom(ladder_preset(5)));
    run(&mut app, 1);
    assert_eq!(visible(&app, &h), 0);
    app.insert_resource(custom(ladder_preset(0)));
    run(&mut app, 1);
    assert_eq!(
        visible(&app, &l) + visible(&app, &h),
        0,
        "the depth fade is back on: the cones wait for the prepass"
    );
    run(&mut app, 80);
    assert_eq!(visible(&app, &l) + visible(&app, &h), 6);
}

#[test]
pub(super) fn the_cap_hides_the_farthest_cones_and_the_stats_count_them() {
    for (cap, spawned, shown) in [(0u32, 20usize, 0usize), (16, 40, 16), (96, 120, 96)] {
        let mut q = ladder_preset(0);
        q.cone_cap = cap;
        let mut app = app(0);
        app.insert_resource(custom(q));
        let cones: Vec<Entity> = (0..spawned)
            .map(|i| {
                spawn_cone(
                    &mut app,
                    ConeKind::Lantern,
                    ConeLevel::Fixed(1.0),
                    5.0 + i as f32,
                )
            })
            .collect();
        run(&mut app, 80);
        assert_eq!(visible(&app, &cones), shown, "cap {cap}");
        // The nearest ones are the visible ones.
        for c in &cones[..shown] {
            assert_eq!(
                *app.world().get::<Visibility>(*c).unwrap(),
                Visibility::Inherited
            );
        }
        let stats = *app.world().resource::<ConeStats>();
        assert_eq!(stats.total, spawned);
        assert_eq!(stats.active, shown);
        if cap > 0 {
            assert_eq!(stats.over_budget, spawned - shown);
        }
    }
}
