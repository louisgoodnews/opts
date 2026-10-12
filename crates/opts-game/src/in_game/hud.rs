//! In-game driving telemetry dashboard, instrument cluster, and HUD controls.

use bevy::prelude::*;
use opts_bevy::{
    camera_hint_text, CameraAvailability, CameraState, PlayerVehicle, VehicleController,
};
use opts_gui::components::Button;
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::*;
use opts_sim::vehicle::inputs::GearPosition;

use crate::app_state::AppState;
use crate::in_game::instruments::{InstrumentInk, InstrumentPanel};
use crate::in_game::types::{
    CameraModeText, GearIndicatorText, HandbrakeIndicatorText, HudClockText, InGameEntity,
    InGameHudRoot, InGameSessionInfo, PauseMenuBtn, SpeedometerText, TachometerText,
};
use crate::menu::glass::GlassSurface;
use bevy::color::Alpha;
use opts_gui::theme::InstrumentVariant;

/// Spawns the in-game driving telemetry overlay HUD.
pub fn spawn_in_game_hud(
    commands: &mut Commands,
    theme: &MaterialTheme,
    session: &InGameSessionInfo,
) {
    // First colours; `instruments::sync_instrument_tokens` keeps them in step with the day/night
    // variant and the theme through the `InstrumentPanel` / `InstrumentInk` markers.
    let ink = *theme.instrument(InstrumentVariant::Day);
    let root = commands
        .spawn((
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
                GlobalZIndex(50),
            ),
            InGameEntity,
            InGameHudRoot,
        ))
        .id();

    // 1. Top Bar: Session Info (Left) and Camera / Exit (Right)
    let top_bar = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .with_children(|bar| {
            // Left: Session Location & Environment Pill
            bar.spawn((
                (
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                    BackgroundColor(
                        theme
                            .scheme
                            .get(ColorRole::SurfaceContainerHigh)
                            .with_alpha(0.85),
                    ),
                ),
                GlassSurface::new(ColorRole::SurfaceContainerHigh, 0.8),
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
            ))
            .with_children(|pill| {
                pill.spawn((
                    (
                        Text::new(format!("{} - {}", session.map_name, session.entry_point)),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(ColorRole::OnSurface)),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                ));

                pill.spawn((
                    (
                        Text::new(format!(
                            "Departure: {:02}:{:02} | Weather: {}",
                            session.start_hour, session.start_minute, session.weather_preset
                        )),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                    ),
                    HudClockText,
                    TypeStyleToken(TypeScale::Label),
                ));
            });

            // Right: Camera Mode & Exit to Menu Button
            bar.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|right_box| {
                // Camera Indicator
                right_box
                    .spawn((
                        (
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                            BackgroundColor(
                                theme
                                    .scheme
                                    .get(ColorRole::SurfaceContainerHigh)
                                    .with_alpha(0.85),
                            ),
                        ),
                        RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                    ))
                    .with_children(|cam_pill| {
                        cam_pill.spawn((
                            (
                                Text::new("--"),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..default()
                                },
                                TextColor(theme.scheme.get(ColorRole::Primary)),
                            ),
                            CameraModeText,
                            TypeStyleToken(TypeScale::Label),
                        ));
                    });

                // Exit Button
                right_box
                    .spawn((
                        (
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(theme.scheme.get(ColorRole::Error)),
                            BackgroundColor(
                                theme.scheme.get(ColorRole::ErrorContainer).with_alpha(0.85),
                            ),
                        ),
                        M3Button::default(),
                        PauseMenuBtn,
                        opts_gui::interaction::focus::PointerOnly,
                        RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            (
                                Text::new("Exit to Menu [Esc]"),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..default()
                                },
                                TextColor(theme.scheme.get(ColorRole::OnErrorContainer)),
                            ),
                            TypeStyleToken(TypeScale::Label),
                        ));
                    });
            });
        })
        .id();

    // 2. Bottom Dashboard: Instrument Cluster & Controls Legend
    let bottom_section = commands
        .spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            })
        .with_children(|dash| {
            // Instrument Cluster Card
            dash.spawn((
                (Node {
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(14.0)),
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(36.0),
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    }, BorderColor::all(ink.panel_border), BackgroundColor(ink.panel)),
                InstrumentPanel,
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Large),
            ))
            .with_children(|cluster| {
                // Speedometer
                cluster
                    .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                    .with_children(|speed_box| {
                        speed_box.spawn((
                            (Text::new("0.0"), TextFont { font_size: FontSize::Px(38.0), ..default() }, TextColor(ink.needle)),
                            SpeedometerText,
                            InstrumentInk::Needle,
                            TypeStyleToken(TypeScale::Headline),
                        ));

                        speed_box.spawn((
                            (Text::new("KM/H"), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(ink.text_dim)),
                            InstrumentInk::TextDim,
                            TypeStyleToken(TypeScale::Label),
                        ));
                    });

                // Tachometer (RPM)
                cluster
                    .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                    .with_children(|rpm_box| {
                        rpm_box.spawn((
                            (Text::new("--"), TextFont { font_size: FontSize::Px(26.0), ..default() }, TextColor(ink.text)),
                            TachometerText,
                            InstrumentInk::Text,
                            TypeStyleToken(TypeScale::Headline),
                        ));

                        rpm_box.spawn((
                            (Text::new("ENGINE RPM"), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(ink.text_dim)),
                            InstrumentInk::TextDim,
                            TypeStyleToken(TypeScale::Label),
                        ));
                    });

                // Transmission Gear Selector
                cluster
                    .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                    .with_children(|gear_box| {
                        gear_box.spawn((
                            (Text::new("--"), TextFont { font_size: FontSize::Px(28.0), ..default() }, TextColor(ink.ok)),
                            GearIndicatorText,
                            InstrumentInk::Ok,
                            TypeStyleToken(TypeScale::Headline),
                        ));

                        gear_box.spawn((
                            (Text::new("GEAR [D/N/R]"), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(ink.text_dim)),
                            InstrumentInk::TextDim,
                            TypeStyleToken(TypeScale::Label),
                        ));
                    });

                // Parking Brake Indicator
                cluster
                    .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                    .with_children(|brake_box| {
                        brake_box.spawn((
                            (Text::new("--"), TextFont { font_size: FontSize::Px(14.0), ..default() }, TextColor(ink.ok)),
                            HandbrakeIndicatorText,
                            InstrumentInk::Ok,
                            TypeStyleToken(TypeScale::BodyMedium),
                        ));

                        brake_box.spawn((
                            (Text::new("PARK [.]"), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(ink.text_dim)),
                            InstrumentInk::TextDim,
                            TypeStyleToken(TypeScale::Label),
                        ));
                    });
            });

            // Driver Controls Legend
            dash.spawn((
                (Node {
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    }, BorderColor::all(ink.panel_border), BackgroundColor(ink.panel)),
                InstrumentPanel,
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Full),
            ))
            .with_children(|legend| {
                legend.spawn((
                    (Text::new("[W/S] Throttle/Brake  •  [←/→] Steer  •  [.] Parking brake  •  [D/N/R] Gears  •  [1-5] Camera  •  [P] Pause  •  [Esc] Menu"), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(ink.text)),
                    InstrumentInk::Text,
                    TypeStyleToken(TypeScale::Label),
                ));
            });
        })
        .id();

    commands
        .entity(root)
        .add_children(&[top_bar, bottom_section]);
}

/// Updates telemetry HUD elements every frame from the active vehicle controller.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_in_game_hud(
    time: Res<Time>,
    vehicle_query: Query<&VehicleController, With<PlayerVehicle>>,
    mut speed_query: Query<
        &mut Text,
        (
            With<SpeedometerText>,
            Without<TachometerText>,
            Without<GearIndicatorText>,
            Without<HandbrakeIndicatorText>,
        ),
    >,
    mut rpm_query: Query<
        &mut Text,
        (
            With<TachometerText>,
            Without<SpeedometerText>,
            Without<GearIndicatorText>,
            Without<HandbrakeIndicatorText>,
        ),
    >,
    mut gear_query: Query<
        &mut Text,
        (
            With<GearIndicatorText>,
            Without<SpeedometerText>,
            Without<TachometerText>,
            Without<HandbrakeIndicatorText>,
        ),
    >,
    mut brake_query: Query<
        (&mut Text, &mut InstrumentInk),
        (
            With<HandbrakeIndicatorText>,
            Without<SpeedometerText>,
            Without<TachometerText>,
            Without<GearIndicatorText>,
        ),
    >,
    mut session: ResMut<InGameSessionInfo>,
    mut clock_query: Query<
        &mut Text,
        (
            With<HudClockText>,
            Without<SpeedometerText>,
            Without<TachometerText>,
            Without<GearIndicatorText>,
            Without<HandbrakeIndicatorText>,
        ),
    >,
) {
    let Ok(controller) = vehicle_query.single() else {
        return;
    };

    // 1. Road speed
    let speed_kmh = controller.forward_speed_km_h().abs();
    for mut text in &mut speed_query {
        text.0 = format!("{:.1}", speed_kmh);
    }

    // 2. Engine RPM: the drivetrain model is not yet parameterised from the vehicle's own
    // configuration (gear ratios and engine curve live in its scripts/consts), so show N/A
    // rather than a default-model number.
    for mut text in &mut rpm_query {
        text.0 = "--".to_string();
    }

    // 3. Transmission gear
    let gear_str = match controller.gear() {
        GearPosition::Drive => "D".to_string(),
        GearPosition::Neutral => "N".to_string(),
        GearPosition::Reverse => "R".to_string(),
        GearPosition::Forward(g) => format!("{g}"),
    };
    for mut text in &mut gear_query {
        text.0 = gear_str.clone();
    }

    // 4. Parking brake
    let is_park = controller.inputs().is_parking_brake_engaged();
    for (mut text, mut ink) in &mut brake_query {
        if is_park {
            text.0 = "[PARK]".to_string();
            *ink = InstrumentInk::Alarm;
        } else {
            text.0 = "RELEASED".to_string();
            *ink = InstrumentInk::Ok;
        }
    }

    // 5. Elapsed simulation clock
    session.sim_elapsed_seconds += time.delta_secs();
    let total_secs = (session.start_hour * 3600 + session.start_minute * 60) as f32
        + session.sim_elapsed_seconds;
    let cur_h = ((total_secs / 3600.0) as u32) % 24;
    let cur_m = (((total_secs % 3600.0) / 60.0) as u32) % 60;
    let cur_s = ((total_secs % 60.0) as u32) % 60;

    for mut text in &mut clock_query {
        text.0 = format!(
            "Time: {:02}:{:02}:{:02} | Weather: {}",
            cur_h, cur_m, cur_s, session.weather_preset
        );
    }
}

/// Handles interactions with HUD buttons (such as Exit to Menu).
pub fn handle_hud_interactions(
    mut events: MessageReader<ButtonClicked>,
    pause_query: Query<&PauseMenuBtn>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for event in events.read() {
        if pause_query.contains(event.0) {
            tracing::info!("User requested exit to Main Menu from in-game HUD");
            next_state.set(AppState::MainMenu);
        }
    }
}

/// Refreshes the camera hint line whenever the camera state or mode availability changes.
pub fn handle_camera_shortcuts(
    state: Res<CameraState>,
    avail: Res<CameraAvailability>,
    vehicle: Query<&opts_bevy::vehicle::BusVehicle, With<PlayerVehicle>>,
    mut text_query: Query<&mut Text, With<CameraModeText>>,
) {
    let total = vehicle.single().map_or(0, |b| match state.mode {
        opts_bevy::CameraMode::Cockpit => b.driver_cameras.len(),
        opts_bevy::CameraMode::Passenger => b.pax_camera_count(),
        _ => 0,
    });
    let line = camera_hint_text(&state, &avail, total);
    for mut text in &mut text_query {
        if text.0 != line {
            text.0.clone_from(&line);
        }
    }
}
