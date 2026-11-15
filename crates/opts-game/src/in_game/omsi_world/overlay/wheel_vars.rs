//! `--debug-wheel-vars`: the wheel animation variables of the player's vehicle, as text.

use bevy::prelude::*;

/// `--debug-wheel-vars`: show the wheel animation variables of the player's vehicle.
#[derive(Resource, Default, Clone, Copy)]
pub struct DebugWheelVars(pub bool);

/// Marker of the wheel-variable overlay text.
#[derive(Component)]
pub struct WheelVarsText;

/// Lists `wheel_rotation_<n>_L/R` and `wheel_rotationspeed_<n>_l/r` (rad, rpm) per axle: the
/// head's axles, then each trailer's under the consist-wide numbers its model uses.
pub fn update_wheel_vars_overlay(
    mut commands: Commands,
    flag: Option<Res<DebugWheelVars>>,
    vehicle: Query<
        (
            &opts_bevy::VehicleController,
            &opts_bevy::VehicleScripts,
            Option<&opts_bevy::ArticulatedChain>,
        ),
        With<opts_bevy::PlayerVehicle>,
    >,
    segments: Query<&opts_bevy::ArticulatedSegment>,
    mut text: Query<&mut Text, With<WheelVarsText>>,
) {
    if !flag.is_some_and(|f| f.0) {
        return;
    }
    let Ok((controller, scripts, chain)) = vehicle.single() else {
        return;
    };
    let Ok(engine) = scripts.engine.lock() else {
        return;
    };
    let mut out = String::from("wheel variables (rad | rpm)\n");
    let line = |label: &str, n: usize| -> String {
        format!(
            "{label} axle {n}: rot L {:.2} R {:.2} | speed l {:.1} r {:.1}\n",
            engine.get(&format!("wheel_rotation_{n}_l")),
            engine.get(&format!("wheel_rotation_{n}_r")),
            engine.get(&format!("wheel_rotationspeed_{n}_l")),
            engine.get(&format!("wheel_rotationspeed_{n}_r")),
        )
    };
    for n in 0..controller.sim.axles.len() {
        out.push_str(&line("head", n));
    }
    if let Some(chain) = chain {
        for (k, &e) in chain.segments.iter().enumerate() {
            out.push_str(&format!(
                "joint {k}: alpha {:.2} beta {:.2} gamma {:.2} deg\n",
                engine.get(&format!("articulation_{k}_alpha")),
                engine.get(&format!("articulation_{k}_beta")),
                engine.get(&format!("articulation_{k}_gamma")),
            ));
            if let Ok(seg) = segments.get(e) {
                for j in 0..seg.wheels.len() {
                    out.push_str(&line(&format!("trailer {}", k + 1), seg.first_axle + j));
                }
            }
        }
    }
    if let Ok(mut t) = text.single_mut() {
        t.0 = out;
    } else {
        commands.spawn((
            Text::new(out),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.6)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(70.0),
                right: Val::Px(12.0),
                ..default()
            },
            WheelVarsText,
            crate::in_game::types::InGameEntity,
        ));
    }
}
