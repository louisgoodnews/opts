//! The fog layer near the ground: where it lies, how it lifts with height and how the sky shows it.

use super::*;

fn ground_fog_conditions(visibility: f32) -> WeatherConditions {
    WeatherConditions {
        fog_visibility_m: visibility,
        fog_density: 1.0,
        ..WeatherConditions::default()
    }
}

#[test]
fn only_low_visibility_presets_get_a_fog_layer() {
    assert_eq!(
        fog_layer_thickness(&ground_fog_conditions(75.0)),
        Some(30.0)
    );
    assert_eq!(
        fog_layer_thickness(&ground_fog_conditions(FOG_LAYER_MAX_VISIBILITY_M)),
        Some(60.0)
    );
    for v in [500.0, 700.0, 6000.0, 50_000.0] {
        assert_eq!(fog_layer_thickness(&ground_fog_conditions(v)), None, "{v}");
    }
    // Very thin fog still has a layer of at least 25 m; a thin factor makes it thinner.
    assert_eq!(
        fog_layer_thickness(&ground_fog_conditions(20.0)),
        Some(25.0)
    );
    let mut c = ground_fog_conditions(150.0);
    c.fog_density = 0.5;
    assert_eq!(fog_layer_thickness(&c), Some(45.0));
}

#[test]
fn inside_the_layer_the_fog_is_the_presets_own() {
    for h in [0.0, 2.0, 15.0] {
        let p = ground_fog(75.0, 30.0, h, 0.0);
        assert_eq!((p.visibility_m, p.sky_clear), (75.0, 0.0), "h={h}");
        let looking_up = ground_fog(75.0, 30.0, h, 40.0);
        assert_eq!(looking_up.visibility_m, 75.0, "h={h}");
    }
}

#[test]
fn the_fog_lifts_smoothly_and_monotonically_with_height() {
    let mut last = ground_fog(75.0, 30.0, 0.0, -10.0);
    let mut h = 0.0;
    while h <= 400.0 {
        h += 0.5;
        let p = ground_fog(75.0, 30.0, h, -10.0);
        assert!(p.visibility_m >= last.visibility_m - 1e-3, "h={h}");
        assert!(p.sky_clear >= last.sky_clear - 1e-6, "h={h}");
        // No popping: a half metre step moves the visibility by a few metres at most.
        assert!(p.visibility_m - last.visibility_m < 3.0, "h={h}");
        last = p;
    }
    let at = |h: f32| ground_fog(75.0, 30.0, h, 0.0);
    let (a, b, c, d) = (at(2.0), at(20.0), at(40.0), at(70.0));
    assert!(a.visibility_m < b.visibility_m && b.visibility_m < c.visibility_m);
    assert!(c.visibility_m < d.visibility_m);
    assert!(d.sky_clear > 0.99 && a.sky_clear == 0.0);
    // From 2·T up the visibility is the layer's share of the path: V · h / T.
    assert!((d.visibility_m - 75.0 * 70.0 / 30.0).abs() < 0.01);
}

#[test]
fn looking_up_from_above_the_layer_reaches_farther_but_not_from_inside() {
    let down = ground_fog(75.0, 30.0, 70.0, -30.0).visibility_m;
    let level = ground_fog(75.0, 30.0, 70.0, 0.0).visibility_m;
    let up = ground_fog(75.0, 30.0, 70.0, 15.0).visibility_m;
    assert_eq!(down, level);
    assert!((up - level * 4.0).abs() < 0.01, "{up} {level}");
    assert!(ground_fog(75.0, 30.0, 70.0, 7.0).visibility_m > level);
    // Never beyond the distance where the fog is off.
    assert!(ground_fog(75.0, 30.0, 5000.0, 20.0).visibility_m < FOG_MAX_VISIBILITY_M);
}

#[test]
fn the_view_follows_the_layer_rule_and_leaves_other_presets_alone() {
    let c = ground_fog_conditions(75.0);
    let low = ground_fog_view(&c, true, 2.0, 0.0);
    let high = ground_fog_view(&c, true, 70.0, 0.0);
    assert_eq!(low.layer_m, Some(30.0));
    assert_eq!(low.effective_visibility_m, 75.0);
    assert!(high.effective_visibility_m > 170.0 && high.sky_clear > 0.99);
    for v in [500.0, 700.0] {
        let c = ground_fog_conditions(v);
        let view = ground_fog_view(&c, true, 70.0, 0.0);
        assert_eq!(view.layer_m, None);
        assert_eq!((view.effective_visibility_m, view.sky_clear), (v, 0.0));
    }
    assert_eq!(ground_fog_view(&c, false, 70.0, 0.0).layer_m, None);
}

#[test]
fn a_cleared_sky_keeps_only_a_hazy_rim_and_the_overcast() {
    let c = ground_fog_conditions(75.0);
    assert_eq!(sky_veil_clear(&c, 0.5, 0.0), sky_veil(&c, 0.5));
    let zenith = sky_veil_clear(&c, 0.0, 1.0);
    let mid = sky_veil_clear(&c, 0.6, 1.0);
    let rim = sky_veil_clear(&c, 1.0, 1.0);
    assert!(zenith == 0.0 && mid == 0.0 && rim > 0.3 && rim < sky_veil(&c, 1.0));
    // The frame's top third of a horizon view (4° … 11° up) keeps only a faint veil.
    let band: Vec<f32> = (0..=14)
        .map(|i| sky_veil_clear(&c, 1.0 - (4.0 + i as f32 * 0.5) / 90.0, 1.0))
        .collect();
    let mean = band.iter().sum::<f32>() / band.len() as f32;
    assert!(band[0] < 0.12 && mean < 0.05, "{band:?} {mean}");
    let mut overcast = c.clone();
    overcast.cloud = Some(CloudSpec {
        name: "Overcast 1".into(),
        texture: None,
        size_m: 1000.0,
        base_m: 100.0,
        cover: CloudCover::Overcast,
        coverage: 1.0,
    });
    assert!(sky_veil_clear(&overcast, 0.0, 1.0) > 0.1);
}
