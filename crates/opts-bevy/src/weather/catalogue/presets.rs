//! The values of the built-in presets: the clouds, precipitation, fog and temperature of each.

use super::*;

/// The built-in preset value of a built-in id (behaves as the five presets always did: clear,
/// overcast and wet/snowy/foggy skies at 15 °C, −3 °C with snow).
#[must_use]
pub fn builtin_preset(id: &str) -> WeatherFile {
    let mut w = WeatherFile::default();
    let overcast = |base_m: f32| {
        Some(CloudLayer {
            kind: "Overcast 1".to_owned(),
            base_m,
        })
    };
    match legacy_id(id) {
        "Overcast Autumn" => {
            w.clouds = overcast(300.0);
            w.temp.dew_point_c = 11.0;
            w.fog.visibility_m = 12_000.0;
        }
        "Rainy Afternoon" => {
            w.clouds = overcast(200.0);
            w.precip.kind = PrecipKind::Rain;
            w.precip.intensity = 130.0;
            w.ground_wet = [78.0, 255.0, 60.0];
            w.temp.dew_point_c = 13.0;
            w.fog.visibility_m = 6000.0;
            w.wind.direction_deg = 230.0;
            w.wind.speed = 4.0;
        }
        "Winter Snowfall" => {
            w.clouds = overcast(250.0);
            w.precip.kind = PrecipKind::Snow;
            w.precip.intensity = 150.0;
            w.snow = true;
            w.snow_on_road = true;
            w.ground_wet = [1.7, 255.0, 0.0];
            w.temp.celsius = -3.0;
            w.temp.dew_point_c = -4.0;
            w.fog.visibility_m = 3000.0;
            w.wind.direction_deg = 300.0;
            w.wind.speed = 1.5;
        }
        "Autumn Morning Fog" => {
            w.clouds = overcast(100.0);
            w.fog = Fog {
                visibility_m: 300.0,
                density: 1.0,
            };
            w.temp.dew_point_c = 15.0;
            w.ground_wet = [30.0, 120.0, 0.0];
        }
        _ => {}
    }
    w
}
