//! The telemetry overlay: its text and the per-frame refresh from the vehicle controller.

use super::*;

/// Marker component for the on-screen telemetry dashboard text.
#[derive(Component)]
pub(super) struct TelemetryHudText;

/// Spawns the telemetry overlay user interface.
pub(super) fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.1, 0.12, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                (
                    Text::new("OPTS Driving Telemetry\nSpeed: 0.0 km/h\nRPM: 600\nGear: D"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.95, 1.0)),
                ),
                TelemetryHudText,
            ));
        });
}

/// Refreshes telemetry text overlay each frame from active [`VehicleController`] state.
pub(super) fn update_telemetry_hud(
    vehicle_query: Query<&VehicleController, With<PlayerVehicle>>,
    mut text_query: Query<&mut Text, With<TelemetryHudText>>,
) {
    let Ok(controller) = vehicle_query.single() else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let speed = controller.forward_speed_km_h();
    let rpm = controller.engine_rpm();
    let gear_str = match controller.gear() {
        opts_sim::vehicle::inputs::GearPosition::Neutral => "N (Neutral)".to_string(),
        opts_sim::vehicle::inputs::GearPosition::Drive => "D (Drive)".to_string(),
        opts_sim::vehicle::inputs::GearPosition::Reverse => "R (Reverse)".to_string(),
        opts_sim::vehicle::inputs::GearPosition::Forward(g) => format!("{g} (Hold)"),
    };
    let park_str = if controller.inputs().is_parking_brake_engaged() {
        "PARK [ON]"
    } else {
        "PARK [OFF]"
    };
    let steer_pct = (controller.inputs().steering() * 100.0).round();

    text.0 = format!(
        "OPTS Driving Simulator\n\
         ─────────────────────────────\n\
         Speed:     {:>5.1} km/h\n\
         Engine:    {:>5.0} RPM\n\
         Gear:      {}\n\
         Brake:     {}\n\
         Steering:  {:>+4.0}%\n\
         ─────────────────────────────\n\
         W / S:     Throttle / Foot Brake\n\
         A / D:     Steer Left / Right\n\
         Space:     Handbrake Toggle\n\
         E / N / R: Gear Select (Drive/Neutral/Rev)\n\
         F1 / F2:   Chase / Cockpit Camera\n\
         Esc:       Exit",
        speed, rpm, gear_str, park_str, steer_pct
    );
}
