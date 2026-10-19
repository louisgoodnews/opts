//! Precipitation, the look, clouds and surfaces.

use super::*;

#[test]
fn precipitation_scales_with_the_rate_and_the_budget() {
    let mut c = WeatherConditions::default();
    assert_eq!(precip_plan(&c, 3000), None);
    c.precip = PrecipKind::Rain;
    c.precip_rate = 0.1;
    let light = precip_plan(&c, 3000).unwrap();
    c.precip_rate = 1.0;
    let heavy = precip_plan(&c, 3000).unwrap();
    assert!(light.quads < heavy.quads);
    assert_eq!(heavy.quads, 3000);
    assert_eq!(precip_plan(&c, 0), None);
    assert!(precip_plan(&c, 1500).unwrap().quads <= 1500);
    c.precip_rate = 0.0;
    assert_eq!(precip_plan(&c, 3000), None, "no rate, no drops");
    // Snow falls slowly.
    c.precip = PrecipKind::Snow;
    c.precip_rate = 0.5;
    let snow = precip_plan(&c, 3000).unwrap();
    assert!(snow.fall_speed < heavy.fall_speed / 3.0);
}

#[test]
fn wind_slants_the_rain_in_the_direction_it_blows() {
    let mut c = WeatherConditions {
        precip: PrecipKind::Rain,
        precip_rate: 0.5,
        wind_speed: 6.0,
        // From the east (90°): the rain is carried to the west (-x).
        wind_from_deg: 90.0,
        ..WeatherConditions::default()
    };
    let p = precip_plan(&c, 1000).unwrap();
    assert!(p.drift.x < -3.0 && p.drift.y.abs() < 1e-3, "{:?}", p.drift);
    // From the north (0°): carried south (+z).
    c.wind_from_deg = 0.0;
    assert!(precip_plan(&c, 1000).unwrap().drift.y > 3.0);
    // A storm cannot make the rain fly sideways.
    c.wind_speed = 40.0;
    let p = precip_plan(&c, 1000).unwrap();
    assert!(p.drift.length() <= p.fall_speed * 1.5 + 1e-4);
}

#[test]
fn the_look_is_scene_linear_and_follows_the_time_of_day() {
    let fog = WeatherConditions {
        fog_visibility_m: 100.0,
        ..WeatherConditions::default()
    };
    let day = weather_look(&fog, [0.6, 0.65, 0.75], 45.0);
    let night = weather_look(&fog, [0.02, 0.02, 0.04], -30.0);
    // sRGB 0.6 is about 0.32 linear: darker than the sRGB number.
    assert!(day.fog[0] < 0.5 && day.fog[0] > 0.2, "{:?}", day.fog);
    assert!(night.fog[0] < 0.01 && night.cloud < 0.06 && night.precip < 0.4);
    assert!((day.cloud - 1.0 + 0.15 * fogginess(&fog)).abs() < 1e-4);
}

#[test]
fn clouds_drift_downwind_and_dim_at_night() {
    let mut c = WeatherConditions {
        wind_from_deg: 90.0,
        wind_speed: 2.0,
        ..WeatherConditions::default()
    };
    let d = cloud_drift(&c);
    assert!((d.x + 6.0).abs() < 1e-4 && d.y.abs() < 1e-4);
    assert!(cloud_tint(&c, -30.0) < 0.06);
    assert!((cloud_tint(&c, 45.0) - 1.0).abs() < 1e-4);
    c.precip_rate = 1.0;
    assert!(cloud_tint(&c, 45.0) < 0.75);
}

#[test]
fn surfaces_get_wet_and_snowy_from_the_flags() {
    let dry = surface_state(&WeatherConditions::default());
    assert_eq!(dry, SurfaceState::default());
    let w = WeatherFile {
        ground_wet: [78.0, 255.0, 115.9],
        ..WeatherFile::default()
    };
    let wet = surface_state(&WeatherConditions::from_file(&w, &clouds()));
    assert!((wet.wet - 1.0).abs() < 1e-6);
    assert!(wet.puddles > 0.4 && wet.puddles <= 0.45);
    let s = WeatherFile {
        ground_wet: [1.7, 255.0, 0.0],
        snow: true,
        snow_on_road: true,
        ..WeatherFile::default()
    };
    let snow = surface_state(&WeatherConditions::from_file(&s, &clouds()));
    assert!(snow.wet < 0.5 && snow.snow_ground > 0.8 && snow.snow_road > 0.6);
    // Only the `[snow]` flag covers the ground.
    let only_road = WeatherFile {
        snow_on_road: true,
        ..WeatherFile::default()
    };
    let o = surface_state(&WeatherConditions::from_file(&only_road, &clouds()));
    assert_eq!((o.snow_ground, o.snow_road > 0.0), (0.0, true));
}

#[test]
fn road_material_darkens_when_wet_and_whitens_under_snow() {
    let base = [0.4, 0.4, 0.4];
    let (c, r) = road_surface(base, 0.9, &SurfaceState::default());
    assert_eq!((c, r), (base, 0.9));
    let wet = SurfaceState {
        wet: 1.0,
        ..SurfaceState::default()
    };
    let (c, r) = road_surface(base, 0.9, &wet);
    assert!(c[0] < 0.4 && r < 0.5);
    let snow = SurfaceState {
        snow_road: 0.7,
        ..SurfaceState::default()
    };
    assert!(road_surface(base, 0.9, &snow).0[0] > 0.7);
}

#[test]
fn precipitation_quality_keys_and_budgets() {
    for q in [
        PrecipitationQuality::Off,
        PrecipitationQuality::Low,
        PrecipitationQuality::Medium,
        PrecipitationQuality::High,
    ] {
        assert_eq!(PrecipitationQuality::parse(q.key()), Some(q));
    }
    assert_eq!(PrecipitationQuality::Off.budget(), 0);
    assert!(PrecipitationQuality::High.budget() > PrecipitationQuality::Low.budget());
    assert_eq!(PrecipitationQuality::parse("ultra"), None);
    assert_eq!(
        WeatherEffects::default().precipitation,
        PrecipitationQuality::Medium
    );
}

#[test]
fn weather_kinds_from_preset_words() {
    assert_eq!(WeatherKind::of_preset("Rainy Afternoon"), WeatherKind::Rain);
    assert_eq!(WeatherKind::of_preset("Sommerlich"), WeatherKind::Clear);
    assert!(WeatherKind::Fog.overcast() && !WeatherKind::Clear.overcast());
}
