//! Script variables of the moving AI vehicles: wheels, indicators, lamps, sound and passengers.

use super::*;

/// Period of the indicator flash (s); half of it on, half off.
const BLINK_PERIOD_S: f32 = 0.7;

/// Wheel rotation and steering of every axle. Steering: the lane's heading at the front axle
/// against the body (positive = right).
fn set_wheel_variables(vars: &mut opts_bevy::AiVariables, wheel_angle: f32, steer: f32) {
    for axle in 0..4 {
        for side in ["l", "r"] {
            vars.set(&format!("wheel_rotation_{axle}_{side}"), wheel_angle);
            let s = if axle == 0 { steer } else { 0.0 };
            vars.set(&format!("axle_steering_{axle}_{side}"), s);
        }
    }
}

/// The AI flag, the commanded indicators and the flashing lamps.
fn set_blinker_variables(
    vars: &mut opts_bevy::AiVariables,
    blinker: opts_traffic::Blinker,
    flash: bool,
) {
    let (left, right) = (
        f32::from(flash && blinker == opts_traffic::Blinker::Left),
        f32::from(flash && blinker == opts_traffic::Blinker::Right),
    );
    vars.set("ai", 1.0);
    vars.set(
        "ai_blinker_l",
        f32::from(blinker == opts_traffic::Blinker::Left),
    );
    vars.set(
        "ai_blinker_r",
        f32::from(blinker == opts_traffic::Blinker::Right),
    );
    vars.set("lights_blinker_l", left);
    vars.set("lights_blinker_r", right);
    // The flasher bulbs and the blink clock the model's lamps read (the lights script sets them
    // for a driven vehicle; an AI vehicle has no script to do it).
    vars.set("lights_blinkgeber", f32::from(flash));
    for n in 1..=4 {
        vars.set(&format!("lights_blinker_l_bulb_{n}"), left);
        vars.set(&format!("lights_blinker_r_bulb_{n}"), right);
    }
}

/// The headlights, the tail lamps and the position lights of an AI vehicle: dipped beam, position
/// and rear lamps on in the dark, no high beam (OMSI's AI lamps are `0` off, `0.5` position, `1`
/// dipped, `2` high beam, the same mapping as `ai_light`).
fn set_lamp_variables(vars: &mut opts_bevy::AiVariables, dark: bool) {
    let on = f32::from(dark);
    vars.set("lights_abbl", on);
    vars.set("lights_stand", on);
    vars.set("lights_rueck", on);
    vars.set("lights_fern", 0.0);
    for n in 1..=2 {
        vars.set(&format!("lights_abbl_bulb_{n}"), on);
    }
}

/// Feeds the variables the AI vehicle models read: wheel rotation and steering from speed and lane
/// curvature, indicators, headlights, brake lights and passengers.
///
/// `ai_light` follows the values the AI scripts of OMSI map to their lamps (`0` off, `0.5` position
/// lights, `1` dipped beam, `2` high beam; `docs/src/architecture/ai-traffic.md`, §2.2).
pub fn update_ai_variables(
    time: Res<Time>,
    state: Res<TrafficState>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    clock: Option<Res<opts_bevy::SceneryClock>>,
    mut vehicles: Query<(
        &mut TrafficVehicle,
        &mut opts_bevy::AiVariables,
        Option<&mut crate::vehicle_sound::AiEvents>,
    )>,
) {
    let dt = time.delta_secs().min(0.2);
    let brightness = clock.as_ref().map(|c| c.envir_brightness());
    let weather = clock.map(|c| (c.temperature_c, c.weather));
    let now = time.elapsed_secs();
    let dark = env.is_some_and(|e| e.night());
    for (mut v, mut vars, events) in &mut vehicles {
        let Some(agent) = state.world.agent(v.id) else {
            continue;
        };
        // A driver who is stuck sounds the horn (`ev_AI_Horn` of the AI sound configuration).
        let (fire, next) = super::honk(
            agent.stuck_s as f32,
            v.honk_after,
            agent.params.rowdy as f32,
        );
        v.honk_after = next;
        if let (true, Some(mut events)) = (fire, events) {
            events.raise("ev_ai_horn");
        }
        let Some(command) = state.world.command(v.id) else {
            continue;
        };
        v.wheel_angle = (v.wheel_angle + agent.speed as f32 / v.wheel_radius * dt)
            % (std::f32::consts::TAU * 64.0);
        set_wheel_variables(&mut vars, v.wheel_angle, agent.steer as f32);
        let flash = (now + v.blink_phase) % BLINK_PERIOD_S < BLINK_PERIOD_S / 2.0;
        set_blinker_variables(&mut vars, command.blinker, flash);
        // Brake lights also while standing (held on the brake).
        let brake = f32::from(command.brake > 0.05 || agent.speed < 0.3);
        vars.set("ai_brakelight", brake);
        vars.set("lights_brems", brake);
        let light = if dark { 1.0 } else { 0.0 };
        vars.set("ai_light", light);
        if let Some(b) = brightness {
            vars.set("envir_brightness", b);
        }
        if let Some((temperature, w)) = weather {
            vars.set("weather_temperature", temperature);
            vars.set("weather_abshum", w.abs_hum);
            vars.set("preciptype", w.precip_type);
            vars.set("preciprate", w.precip_rate);
        }
        vars.set("lights_ai", light);
        set_lamp_variables(&mut vars, dark);
        // Sound variables (M-0140): speed and pedals, the engine speed of a gear model, the wheel
        // speeds and the road state.
        let wet = weather.as_ref().is_some_and(|(_, w)| w.precip_rate > 0.05);
        crate::vehicle_sound::publish_sound_variables(
            &mut vars,
            agent.speed as f32,
            agent.accel as f32,
            command.brake,
            v.wheel_radius,
            wet,
            agent.params.class != opts_traffic::VehicleClass::Car,
        );
        for i in 0..4 {
            vars.set(&format!("ai_pax{}", i + 1), f32::from(v.pax >> i & 1));
        }
    }
}
