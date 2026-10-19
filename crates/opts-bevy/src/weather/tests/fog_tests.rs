//! Fog, ground fog, sky veil and light scales.

use super::*;

#[test]
fn fog_is_off_when_the_air_is_clear_and_denser_for_shorter_visibility() {
    let mut c = WeatherConditions::default();
    assert_eq!(fog_density(&c), None);
    c.fog_visibility_m = 14_000.0;
    let thin = fog_density(&c).unwrap();
    c.fog_visibility_m = 75.0;
    let thick = fog_density(&c).unwrap();
    assert!(thick > thin * 100.0, "{thin} {thick}");
    // At the visibility the fog's transmittance is the contrast threshold (2 %).
    let transmittance = (-(thick * 75.0).powi(2)).exp();
    assert!((transmittance - 0.02).abs() < 1e-4, "{transmittance}");
    // A thin layer (0.5) at the same visibility is lighter.
    c.fog_density = 0.5;
    assert!(fog_density(&c).unwrap() < thick);
}

#[test]
fn fog_colour_greys_in_thick_fog_and_keeps_the_horizon_in_clear_air() {
    let horizon = [0.5, 0.6, 0.9];
    let clear = WeatherConditions::default();
    assert_eq!(fog_color(horizon, &clear), horizon);
    let mut fog = clear;
    fog.fog_visibility_m = 80.0;
    let f = fog_color(horizon, &fog);
    let spread = |c: [f32; 3]| c[2] - c[0];
    assert!(spread(f) < spread(horizon) * 0.6, "{f:?}");
    // The colour darkens with the horizon at night.
    let night = fog_color([0.02, 0.02, 0.05], &fog);
    assert!(night.iter().all(|v| *v < 0.06));
}

/// Horizon colours (sRGB) for rising sun elevation: night blue, dawn red, sunset orange, olive
/// dusk, golden hour, pale day blue. Their luminance rises step by step.
const HORIZONS_BY_ELEVATION: [[f32; 3]; 6] = [
    [0.02, 0.03, 0.08],
    [0.30, 0.12, 0.10],
    [0.75, 0.40, 0.15],
    [0.62, 0.62, 0.25],
    [0.95, 0.80, 0.50],
    [0.80, 0.88, 1.00],
];

fn saturation(c: [f32; 3]) -> f32 {
    let hi = c.iter().copied().fold(0.0_f32, f32::max);
    let lo = c.iter().copied().fold(f32::MAX, f32::min);
    if hi <= 1e-6 {
        0.0
    } else {
        (hi - lo) / hi
    }
}

fn luma(c: [f32; 3]) -> f32 {
    c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11
}

#[test]
fn fog_is_never_strongly_tinted_whatever_the_horizon() {
    let cat = installed();
    for id in ["Bodennebel", "Daemmerungsnebel", "Schmuddelwetter", "Sturm"] {
        let c = cat.resolve(id);
        for horizon in HORIZONS_BY_ELEVATION {
            let f = fog_color(horizon, &c);
            let cap =
                1.0 - (1.0 - FOG_MAX_SATURATION) * fogginess(&c).max(effective_cover(&c) * 0.5);
            assert!(
                saturation(f) <= cap + 1e-3,
                "{id} {horizon:?} -> {f:?} saturation {} > {cap}",
                saturation(f)
            );
        }
    }
    // Thick fog (75 m) is grey-white even from an orange sunset horizon.
    let thick = WeatherConditions {
        fog_visibility_m: 75.0,
        ..Default::default()
    };
    let f = fog_color([0.75, 0.40, 0.15], &thick);
    assert!(saturation(f) <= FOG_MAX_SATURATION + 0.01, "{f:?}");
    assert!(f[0] - f[2] < 0.12, "{f:?}");
}

#[test]
fn fog_luminance_rises_with_the_sun_and_never_exceeds_the_horizon() {
    let thick = WeatherConditions {
        fog_visibility_m: 75.0,
        ..Default::default()
    };
    let mut prev = -1.0;
    for horizon in HORIZONS_BY_ELEVATION {
        let l = luma(fog_color(horizon, &thick));
        assert!(l > prev, "{horizon:?}: {l} <= {prev}");
        assert!(
            l <= luma(horizon) + 1e-4,
            "fog brighter than the sky it comes from"
        );
        prev = l;
    }
    // Clear air: the horizon unchanged.
    let clear = WeatherConditions::default();
    for horizon in HORIZONS_BY_ELEVATION {
        assert_eq!(fog_color(horizon, &clear), horizon);
    }
}

#[test]
fn the_sky_is_veiled_by_fog_everywhere_and_by_overcast_towards_the_horizon() {
    let clear = WeatherConditions::default();
    assert_eq!(sky_veil(&clear, 1.0), 0.0);
    let mut fog = clear;
    fog.fog_visibility_m = 100.0;
    assert!(sky_veil(&fog, 0.0) > 0.85 && sky_veil(&fog, 1.0) > 0.85);
    let over = WeatherCatalogue::builtin().resolve("Overcast Autumn");
    assert!(sky_veil(&over, 0.0) < sky_veil(&over, 1.0));
    assert!(sky_veil(&over, 1.0) <= 0.45 + 1e-6);
}

#[test]
fn light_scales_cover_the_documented_range() {
    let clear = light_scales(&WeatherConditions::default());
    assert_eq!((clear.sun, clear.ambient), (1.0, 1.0));
    let cat = installed();
    let storm = light_scales(&cat.resolve("Schmuddelwetter"));
    assert!(storm.sun < 0.6 && storm.sun >= 0.55 - 1e-3, "{}", storm.sun);
    // Cover never darkens below the overcast values.
    let mut c = cat.resolve("Schmuddelwetter");
    c.precip_rate = 1.0;
    c.fog_visibility_m = 5.0;
    assert!(light_scales(&c).sun >= 0.55 - 1e-3);
}

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
