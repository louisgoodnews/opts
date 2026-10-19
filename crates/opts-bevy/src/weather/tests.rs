//! Unit tests of `weather.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;
use opts_core::formats::weather::{CloudType, WeatherText};

fn clouds() -> CloudTypes {
    CloudTypes(vec![
        CloudType {
            name: "Cumulus 1".into(),
            texture: "Cumulus_1.tga".into(),
            size_m: 2000.0,
            cover: CloudCover::Scattered,
        },
        CloudType {
            name: "Overcast 1".into(),
            texture: "OVC_1.bmp".into(),
            size_m: 1000.0,
            cover: CloudCover::Overcast,
        },
    ])
}

fn preset(id: &str, w: WeatherFile, en: &str, de: &str) -> PresetFile {
    let text = |n: &str| WeatherText {
        name: n.into(),
        description: format!("{n} text"),
    };
    PresetFile {
        id: id.into(),
        weather: w,
        texts: [("ENG".to_owned(), text(en)), ("DEU".to_owned(), text(de))].into(),
    }
}

fn installed() -> WeatherCatalogue {
    let rain = WeatherFile {
        precip: opts_core::formats::weather::Precip {
            kind: PrecipKind::Rain,
            intensity: 173.0,
            extra: [0.0; 3],
        },
        clouds: Some(CloudLayer {
            kind: "Overcast 1".into(),
            base_m: 200.0,
        }),
        fog: Fog {
            visibility_m: 200.0,
            density: 1.0,
        },
        ..WeatherFile::default()
    };
    let mut presets = vec![
        preset(
            "#CAVOK",
            WeatherFile::default(),
            "Best Weather",
            "Bestes Wetter",
        ),
        preset("Schmuddelwetter", rain, "Grey and wet", "Schmuddelwetter"),
        preset(
            "Sommerlich",
            WeatherFile::default(),
            "Summerlike",
            "Sommerlich",
        ),
    ];
    presets.sort_by(|a, b| a.id.cmp(&b.id));
    WeatherCatalogue::from_folder(
        PathBuf::from("/omsi"),
        WeatherFolder {
            presets,
            clouds: clouds(),
        },
    )
}

#[test]
fn builtin_presets_keep_the_light_and_temperature_of_the_old_kinds() {
    let cat = WeatherCatalogue::builtin();
    for (id, sun, ambient, temp) in [
        ("Clear Summer Day", 1.0, 1.0, 15.0),
        ("Overcast Autumn", 0.55, 0.8, 15.0),
        ("Rainy Afternoon", 0.55, 0.8, 15.0),
        ("Winter Snowfall", 0.55, 0.8, -3.0),
        ("Autumn Morning Fog", 0.55, 0.8, 15.0),
    ] {
        let c = cat.resolve(id);
        let l = light_scales(&c);
        assert!((l.sun - sun).abs() < 1e-3, "{id} sun {}", l.sun);
        assert!(
            (l.ambient - ambient).abs() < 1e-3,
            "{id} ambient {}",
            l.ambient
        );
        assert_eq!(c.temperature_c, temp, "{id}");
        assert_eq!(c.kind(), WeatherKind::of_preset(id), "{id}");
    }
}

#[test]
fn without_an_install_the_five_ids_are_listed() {
    let cat = WeatherCatalogue::builtin();
    assert_eq!(cat.ids(), BUILTIN_IDS.map(String::from));
    assert!(!cat.is_installed());
    assert_eq!(cat.canonical_id("Rainy Afternoon"), "Rainy Afternoon");
    // An unknown id behaves like the words it contains, else like clear weather.
    assert_eq!(cat.canonical_id("Heavy snow day"), "Winter Snowfall");
    assert_eq!(cat.canonical_id("Thunderstorm & Gale"), "Clear Summer Day");
    assert_eq!(cat.canonical_id(""), "Clear Summer Day");
}

#[test]
fn old_ids_alias_to_the_closest_installed_preset() {
    let cat = installed();
    assert_eq!(cat.ids(), ["#CAVOK", "Schmuddelwetter", "Sommerlich"]);
    assert_eq!(cat.canonical_id("Clear Summer Day"), "#CAVOK");
    assert_eq!(cat.canonical_id("Rainy Afternoon"), "Schmuddelwetter");
    // No installed preset in this catalogue is close to snow or fog: the built-in one stays.
    assert_eq!(cat.canonical_id("Winter Snowfall"), "Winter Snowfall");
    // The overcast id has no alias at all.
    assert_eq!(cat.canonical_id("Overcast Autumn"), "Overcast Autumn");
    assert_eq!(cat.canonical_id("sommerlich"), "Sommerlich");
    // The resolved conditions are the installed ones.
    let c = cat.resolve("Rainy Afternoon");
    assert_eq!(c.fog_visibility_m, 200.0);
    assert!((c.precip_rate - 173.0 / 255.0).abs() < 1e-6);
    // And the built-in ones when no alias exists.
    assert_eq!(cat.resolve("Winter Snowfall").temperature_c, -3.0);
}

#[test]
fn names_follow_the_ui_language() {
    let cat = installed();
    assert_eq!(
        cat.display_name("Sommerlich", "de").as_deref(),
        Some("Sommerlich")
    );
    assert_eq!(
        cat.display_name("#CAVOK", "de").as_deref(),
        Some("Bestes Wetter")
    );
    assert_eq!(
        cat.display_name("#CAVOK", "en").as_deref(),
        Some("Best Weather")
    );
    // A language OMSI has no text for falls back to English.
    assert_eq!(
        cat.display_name("#CAVOK", "ja").as_deref(),
        Some("Best Weather")
    );
    assert!(WeatherCatalogue::builtin()
        .display_name("Clear Summer Day", "en")
        .is_none());
}

#[test]
fn find_accepts_ids_slugs_names_words_and_old_ids() {
    let cat = installed();
    assert_eq!(
        cat.find("schmuddelwetter").as_deref(),
        Some("Schmuddelwetter")
    );
    assert_eq!(cat.find("Best Weather").as_deref(), Some("#CAVOK"));
    assert_eq!(cat.find("bestes-wetter").as_deref(), Some("#CAVOK"));
    assert_eq!(cat.find("grey").as_deref(), Some("Schmuddelwetter"));
    assert_eq!(
        cat.find("rainy-afternoon").as_deref(),
        Some("Schmuddelwetter")
    );
    assert_eq!(cat.find("rain").as_deref(), Some("Schmuddelwetter"));
    assert_eq!(cat.find("clear").as_deref(), Some("#CAVOK"));
    assert_eq!(cat.find("hail"), None);
    let b = WeatherCatalogue::builtin();
    assert_eq!(b.find("rain").as_deref(), Some("Rainy Afternoon"));
    assert_eq!(
        b.find("winter-snowfall").as_deref(),
        Some("Winter Snowfall")
    );
    assert_eq!(
        b.find("autumn-morning-fog").as_deref(),
        Some("Autumn Morning Fog")
    );
}

#[test]
fn conditions_from_a_file() {
    let w = WeatherFile {
        clouds: Some(CloudLayer {
            kind: "Cumulus 3".into(),
            base_m: 300.0,
        }),
        ..WeatherFile::default()
    };
    let c = WeatherConditions::from_file(&w, &clouds());
    let cl = c.cloud.unwrap();
    // The type is not in this catalogue: no texture, scattered, coverage from the digit.
    assert_eq!(cl.texture, None);
    assert_eq!(cl.cover, CloudCover::Scattered);
    assert!((cl.coverage - 0.55).abs() < 1e-6);
    let c1 = WeatherConditions::from_file(
        &WeatherFile {
            clouds: Some(CloudLayer {
                kind: "Cumulus 1".into(),
                base_m: 100.0,
            }),
            ..WeatherFile::default()
        },
        &clouds(),
    );
    assert_eq!(
        c1.cloud.as_ref().unwrap().texture.as_deref(),
        Some("Cumulus_1.tga")
    );
    assert!((c1.cloud.unwrap().coverage - 0.25).abs() < 1e-6);
}

#[test]
fn humidity_follows_the_dew_point() {
    // 15 °C with a 5 °C dew point: about 6.6 g/m³.
    assert!((absolute_humidity(15.0, 5.0) - 6.56).abs() < 0.1);
    // Saturated air at 20 °C holds about 17.3 g/m³.
    assert!((absolute_humidity(20.0, 20.0) - 17.3).abs() < 0.3);
    // A dew point above the temperature is capped.
    assert_eq!(absolute_humidity(10.0, 30.0), absolute_humidity(10.0, 10.0));
    let c = WeatherConditions::default();
    assert!((c.abs_humidity() - 6.56).abs() < 0.1);
}

#[test]
fn script_variables_of_rain_and_dry_weather() {
    let dry = WeatherConditions::default().vars();
    assert_eq!((dry.precip_type, dry.precip_rate), (0.0, 0.0));
    let rain = installed().resolve("Schmuddelwetter").vars();
    assert_eq!(rain.precip_type, 1.0);
    assert!((rain.precip_rate - 173.0 / 255.0).abs() < 1e-6);
    let snow = WeatherCatalogue::builtin()
        .resolve("Winter Snowfall")
        .vars();
    assert_eq!(snow.precip_type, 2.0);
}

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
