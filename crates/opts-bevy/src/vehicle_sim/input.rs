//! Keyboard driver input ingestion for the player's vehicle.

use bevy::prelude::*;
use opts_gui::components::toast::ToastBuilder;
use opts_gui::l10n::Localization;
use opts_sim::vehicle::inputs::GearPosition;

use super::controller::{PlayerVehicle, VehicleController, VehicleInputConfig};

/// What the keyboard input system reads and writes of the player's vehicle.
type PlayerInput<'a> = (
    &'a mut VehicleController,
    Option<&'a VehicleInputConfig>,
    Option<&'a mut crate::rail_controls::RailControls>,
);

/// Systems that poll desktop keyboard inputs and translate them to [`opts_sim::vehicle::inputs::VehicleInputs`].
pub fn vehicle_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<crate::action_keys::ActionKeys>>,
    time: Res<Time>,
    mut remote: ResMut<crate::vehicle_remote::RemoteControls>,
    mut query: Query<PlayerInput, With<PlayerVehicle>>,
    mut commands: Commands,
    loc: Option<Res<Localization>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    let default_config = VehicleInputConfig::default();

    for (mut controller, config_opt, rail) in &mut query {
        let config = config_opt.unwrap_or(&default_config);
        // The refuel action (M-0123) fills the vehicle's fuel model; a rail vehicle has one too.
        // A refused refuel (the vehicle moves) says so in a toast (M-0123).
        if crate::action_keys::resolve(&action_keys).just_pressed(&keyboard, "refuel")
            && !controller.sim.refuel()
        {
            let text = loc
                .as_deref()
                .and_then(|l| l.format("hud-refuel-refused", &[]))
                .unwrap_or_else(|| "Refuel refused: stop the vehicle first.".to_owned());
            ToastBuilder::new(text).spawn(&mut commands);
        }
        let inputs = controller.inputs_mut();
        // A rail vehicle has a power lever and a direction selector, no pedals, no parking brake
        // and no steering (`crate::rail_controls`).
        if let Some(mut rail) = rail {
            crate::rail_controls::apply_keys(
                &mut rail,
                inputs,
                &keyboard,
                crate::action_keys::resolve(&action_keys),
                dt,
            );
            remote.apply(inputs, dt);
            continue;
        }

        // 1. Throttle pedal (W or ArrowUp)
        let throttle_pressed =
            keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp);
        let current_throttle = inputs.throttle();
        if throttle_pressed {
            inputs.set_throttle((current_throttle + config.throttle_rate * dt).min(1.0));
        } else {
            inputs.set_throttle((current_throttle - config.throttle_rate * 2.5 * dt).max(0.0));
        }

        // 2. Service brake pedal (S or ArrowDown)
        let brake_pressed = keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown);
        let current_brake = inputs.service_brake();
        if brake_pressed {
            inputs.set_service_brake((current_brake + config.brake_rate * dt).min(1.0));
        } else {
            inputs.set_service_brake((current_brake - config.brake_rate * 2.5 * dt).max(0.0));
        }

        // 3. Steering wheel (ArrowLeft/ArrowRight; A and D belong to the gear selector)
        let steer_left = keyboard.pressed(KeyCode::ArrowLeft);
        let steer_right = keyboard.pressed(KeyCode::ArrowRight);
        let current_steer = inputs.steering();

        if steer_left && !steer_right {
            inputs.set_steering((current_steer - config.steering_rate * dt).max(-1.0));
        } else if steer_right && !steer_left {
            inputs.set_steering((current_steer + config.steering_rate * dt).min(1.0));
        } else if current_steer > 0.0 {
            inputs.set_steering((current_steer - config.steering_return_rate * dt).max(0.0));
        } else if current_steer < 0.0 {
            inputs.set_steering((current_steer + config.steering_return_rate * dt).min(0.0));
        }

        // 4. Parking brake toggle (the `parking_brake` action, `.` by default)
        if crate::action_keys::resolve(&action_keys).just_pressed(&keyboard, "parking_brake") {
            let current_park = inputs.is_parking_brake_engaged();
            inputs.set_parking_brake(!current_park);
        }

        // 5. Gear selector (D = Drive, N = Neutral, R = Reverse; digits belong to the camera modes)
        if keyboard.just_pressed(KeyCode::KeyD) {
            inputs.set_gear(GearPosition::Drive);
        } else if keyboard.just_pressed(KeyCode::KeyN) {
            inputs.set_gear(GearPosition::Neutral);
        } else if keyboard.just_pressed(KeyCode::KeyR) {
            inputs.set_gear(GearPosition::Reverse);
        }

        // 6. Remote controls (control protocol `drive.*`): held pedals override the keyboard on
        // their axis, gear and parking brake requests are applied once. Same system, same gates.
        remote.apply(inputs, dt);
    }
}
