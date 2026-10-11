//! Atmospheric preset metadata, device location weather lookup, and center column display.

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::prelude::*;

use crate::new_game_screen::types::{
    ApplyLiveWeatherBtn, DeviceLocationWeather, NewGameSetup, RefreshWeatherBtn,
};

/// Detailed physical and atmospheric parameters for a simulation weather preset.
#[derive(Debug, Clone)]
pub struct WeatherPresetMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub temp_str: &'static str,
    pub description: &'static str,
    pub air_temp_c: f32,
    pub wind_kmh: f32,
    pub road_friction: f32,
    pub visibility_m: u32,
    pub precipitation: &'static str,
}

/// Catalog of all atmospheric simulation presets.
pub const ALL_WEATHER_PRESETS: &[WeatherPresetMeta] = &[
    WeatherPresetMeta {
        id: "Clear Summer Day",
        name: "Clear Summer Day",
        icon: "",
        temp_str: "24°C",
        description: "Optimal dry summer conditions with crystal-clear visibility and full solar warmth. Perfect asphalt traction and normal passenger cooling requirements.",
        air_temp_c: 24.0,
        wind_kmh: 8.0,
        road_friction: 1.00,
        visibility_m: 10000,
        precipitation: "None (0.0 mm/h)",
    },
    WeatherPresetMeta {
        id: "Overcast Autumn",
        name: "Overcast & Breezy",
        icon: "",
        temp_str: "14°C",
        description: "Solid cloud cover with gusty winds and cool ambient temperature. Low solar irradiance, dry pavement with standard braking performance.",
        air_temp_c: 14.0,
        wind_kmh: 22.0,
        road_friction: 0.95,
        visibility_m: 8000,
        precipitation: "None (Overcast Stratocumulus)",
    },
    WeatherPresetMeta {
        id: "Rainy Afternoon",
        name: "Rain & Wet Track",
        icon: "",
        temp_str: "11°C",
        description: "Continuous precipitation creating wet road surfaces. Requires windshield wipers; braking distances increased by 30-40% with risk of hydroplaning.",
        air_temp_c: 11.0,
        wind_kmh: 18.0,
        road_friction: 0.70,
        visibility_m: 3500,
        precipitation: "Moderate Rain (4.8 mm/h)",
    },
    WeatherPresetMeta {
        id: "Winter Snowfall",
        name: "Winter Snowfall",
        icon: "",
        temp_str: "-2°C",
        description: "Freezing sub-zero temperatures with steady snowfall. Reduced road grip, wheel slip on acceleration, and active bus interior cabin heating demand.",
        air_temp_c: -2.0,
        wind_kmh: 15.0,
        road_friction: 0.45,
        visibility_m: 1200,
        precipitation: "Dry Snow (2.5 cm/h)",
    },
    WeatherPresetMeta {
        id: "Night Service",
        name: "Night Operation",
        icon: "",
        temp_str: "10°C",
        description: "Calm nocturnal atmosphere. Complete darkness outside vehicle lighting cones; demands headlights, passenger saloon lights, and timetable illumination.",
        air_temp_c: 10.0,
        wind_kmh: 6.0,
        road_friction: 0.98,
        visibility_m: 10000,
        precipitation: "None (Clear Night)",
    },
    WeatherPresetMeta {
        id: "Thunderstorm & Gale",
        name: "Thunderstorm & Gale",
        icon: opts_gui::icons::THUNDERSTORM,
        temp_str: "16°C",
        description: "Severe meteorological event with squalls, crosswinds, and standing water. Maximum wiper speed required and extreme deceleration distances.",
        air_temp_c: 16.0,
        wind_kmh: 58.0,
        road_friction: 0.55,
        visibility_m: 800,
        precipitation: "Heavy Torrential (15.2 mm/h)",
    },
    WeatherPresetMeta {
        id: "Autumn Morning Fog",
        name: "Autumn Morning Fog",
        icon: opts_gui::icons::FOGGY,
        temp_str: "8°C",
        description: "Dense radiation mist reducing sight distances significantly. Demands vehicle fog lamps and reduced operational timetable speeds.",
        air_temp_c: 8.0,
        wind_kmh: 4.0,
        road_friction: 0.85,
        visibility_m: 180,
        precipitation: "Mist / High Relative Humidity (98%)",
    },
    WeatherPresetMeta {
        id: "Real-Time Local Weather",
        name: "Live Device Location",
        icon: opts_gui::icons::LOCATION_ON,
        temp_str: "Live",
        description: "Synchronizes the in-game environmental simulator with real-time live meteorological telemetry retrieved for your local computer device location.",
        air_temp_c: 12.0,
        wind_kmh: 26.0,
        road_friction: 0.90,
        visibility_m: 6000,
        precipitation: "Real-Time Sensor Telemetry",
    },
];

/// Returns the metadata for a preset name.
pub fn get_preset_meta(name: &str) -> &'static WeatherPresetMeta {
    ALL_WEATHER_PRESETS
        .iter()
        .find(|p| p.id == name || p.name == name)
        .unwrap_or(&ALL_WEATHER_PRESETS[0])
}

/// Spawns a background worker thread to query live weather based on IP / device location.
pub fn query_live_weather_background(weather_res: Arc<Mutex<DeviceLocationWeather>>) {
    thread::spawn(move || {
        let output = Command::new("curl")
            .args([
                "-s",
                "--max-time",
                "4",
                "https://wttr.in/?format=%l|%C|%t|%w|%h",
            ])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let parts: Vec<&str> = text.split('|').collect();
            if parts.len() >= 5 {
                let location = parts[0].trim().to_string();
                let condition = parts[1].trim().to_string();
                let temp_raw = parts[2].trim().replace(['+', '°', 'C', ' '], "");
                let wind_raw = parts[3].trim().replace(
                    [
                        '↗', '↖', '↘', '↙', '→', '←', '↑', '↓', 'k', 'm', '/', 'h', ' ',
                    ],
                    "",
                );
                let hum_raw = parts[4].trim().replace(['%', ' '], "");

                let temp: f32 = temp_raw.parse().unwrap_or(12.0);
                let wind: f32 = wind_raw.parse().unwrap_or(26.0);
                let humidity: u32 = hum_raw.parse().unwrap_or(91);

                if let Ok(mut lock) = weather_res.lock() {
                    lock.location_name = location;
                    lock.condition_desc = condition;
                    lock.temperature_c = temp;
                    lock.wind_speed_kmh = wind;
                    lock.humidity_pct = humidity;
                    lock.is_loaded = true;
                }
                return;
            }
        }

        // Fallback default if network was unreachable
        if let Ok(mut lock) = weather_res.lock() {
            if !lock.is_loaded {
                lock.location_name = "Duisburg, North Rhine-Westphalia, DE".to_string();
                lock.condition_desc = "Overcast with light breeze".to_string();
                lock.temperature_c = 12.0;
                lock.wind_speed_kmh = 26.0;
                lock.humidity_pct = 91;
                lock.is_loaded = true;
            }
        }
    });
}

/// Populates the center column with detailed atmospheric parameters and live weather lookup.
pub fn populate_weather_details_inner(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    setup: &NewGameSetup,
    live_weather: &DeviceLocationWeather,
) {
    let preset = get_preset_meta(&setup.weather_preset);

    // 1. Large Preset Headline
    parent.spawn((
        (
            Text::new(preset.name),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(theme.scheme.get(ColorRole::Primary)),
        ),
        TypeStyleToken(TypeScale::HeadlineMedium),
        ForegroundRole(ColorRole::Primary),
    ));

    // 2. Preset Description Body
    parent.spawn((
        (
            Text::new(preset.description),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme.scheme.get(ColorRole::OnSurface)),
        ),
        TypeStyleToken(TypeScale::BodySmall),
        ForegroundRole(ColorRole::OnSurface),
    ));

    // 3. Meteorological Specifications Matrix
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLow)),
        ))
        .with_children(|matrix| {
            matrix.spawn((
                (
                    Text::new("Atmospheric & Physics Parameters:"),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::Primary)),
                ),
                TypeStyleToken(TypeScale::LabelLarge),
                ForegroundRole(ColorRole::Primary),
            ));

            let temp_disp = if setup.weather_preset == "Real-Time Local Weather" {
                format!("{:.1}°C (Live)", live_weather.temperature_c)
            } else {
                format!("{:.1}°C", preset.air_temp_c)
            };

            let wind_disp = if setup.weather_preset == "Real-Time Local Weather" {
                format!("{:.0} km/h (Live)", live_weather.wind_speed_kmh)
            } else {
                format!("{:.0} km/h", preset.wind_kmh)
            };

            spawn_metric_row(matrix, theme, "Ambient Air Temperature", &temp_disp);
            spawn_metric_row(matrix, theme, "Wind Velocity", &wind_disp);
            spawn_metric_row(
                matrix,
                theme,
                "Road Friction Coefficient",
                &format!("{:.2} μ", preset.road_friction),
            );
            spawn_metric_row(
                matrix,
                theme,
                "Optical Sight Distance",
                &format!("{} m", preset.visibility_m),
            );
            spawn_metric_row(matrix, theme, "Precipitation Model", preset.precipitation);
        });

    // 4. Live Device Location Weather Section
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(theme.scheme.get(ColorRole::Primary)),
            BackgroundColor(theme.scheme.get(ColorRole::PrimaryContainer)),
        ))
        .with_children(|box_node| {
            box_node.spawn((
                (
                    Text::new("Live Device Location Telemetry (wttr.in):"),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnPrimaryContainer)),
                ),
                TypeStyleToken(TypeScale::TitleSmall),
                ForegroundRole(ColorRole::OnPrimaryContainer),
            ));

            box_node.spawn((
                (
                    Text::new(format!(
                        "Location: {}\nConditions: {} · {:.1}°C · Wind: {:.0} km/h · Humidity: {}%",
                        live_weather.location_name,
                        live_weather.condition_desc,
                        live_weather.temperature_c,
                        live_weather.wind_speed_kmh,
                        live_weather.humidity_pct
                    )),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnPrimaryContainer)),
                ),
                TypeStyleToken(TypeScale::BodySmall),
                ForegroundRole(ColorRole::OnPrimaryContainer),
            ));

            box_node
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|btns| {
                    btns.spawn((
                        (
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(theme.scheme.get(ColorRole::Primary)),
                            BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
                        ),
                        M3Button::default(),
                        RefreshWeatherBtn,
                        RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                        Focusable { tab_index: 0 },
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            (
                                Text::new("Refresh"),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(theme.scheme.get(ColorRole::Primary)),
                            ),
                            TypeStyleToken(TypeScale::LabelSmall),
                        ));
                    });

                    btns.spawn((
                        (
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(theme.scheme.get(ColorRole::Primary)),
                            BackgroundColor(theme.scheme.get(ColorRole::Primary)),
                        ),
                        M3Button::default(),
                        ApplyLiveWeatherBtn,
                        RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                        Focusable { tab_index: 0 },
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            (
                                Text::new("Apply Live Weather"),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(theme.scheme.get(ColorRole::OnPrimary)),
                            ),
                            TypeStyleToken(TypeScale::LabelSmall),
                        ));
                    });
                });
        });
}

fn spawn_metric_row(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    label: &str,
    val: &str,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                (
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                ),
                TypeStyleToken(TypeScale::BodySmall),
            ));

            row.spawn((
                (
                    Text::new(val),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurface)),
                ),
                TypeStyleToken(TypeScale::LabelMedium),
            ));
        });
}
