//! Fog, ground fog, sky veil and light scales.

use super::*;
mod fog_layer_tests;

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
