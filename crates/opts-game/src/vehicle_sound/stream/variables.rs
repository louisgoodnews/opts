//! Publishing the sound variables of an AI vehicle, and the level and speed of a sound definition from them.

use super::*;

/// Writes the sound variables of one AI vehicle into its table.
pub fn publish_sound_variables(
    vars: &mut AiVariables,
    speed_ms: f32,
    accel_ms2: f32,
    brake: f32,
    wheel_radius: f32,
    wet: bool,
    heavy: bool,
) {
    let kmh = speed_ms * 3.6;
    let throttle = if speed_ms > 0.3 {
        (accel_ms2 / 1.2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let rpm = ai_engine_rpm(kmh, heavy);
    vars.set("velocity", kmh);
    vars.set("velocity_ground", kmh);
    vars.set("throttle", throttle);
    vars.set("engine_throttle_injection", throttle);
    vars.set("brake", brake.clamp(0.0, 1.0));
    vars.set("ai_engine", 1.0);
    vars.set("engine_n", rpm);
    vars.set("ai_motor", rpm);
    vars.set("antrieb_n_kardanwelle", rpm);
    vars.set("streetcond", f32::from(wet));
    // The state of the drive for the conditions: running, not starting, going forward in a gear
    // of the stand-in model (`0` neutral at a standstill), wipers while it rains, electrics on.
    vars.set("engine_on", 1.0);
    vars.set("engine_starter", 0.0);
    vars.set("antrieb_wendesatz", 1.0);
    vars.set("antrieb_getr_aktugang", ai_gear(kmh, heavy));
    vars.set("wiperrunning", f32::from(wet));
    vars.set("elec_busbar_main_sw", 1.0);
    // Wheel speed in rpm from the rolling radius.
    let wheel_rpm = speed_ms / wheel_radius.max(0.05) * 60.0 / std::f32::consts::TAU;
    vars.set("n_wheel", wheel_rpm);
    for axle in 0..4 {
        for side in ["l", "r"] {
            vars.set(&format!("wheel_rotationspeed_{axle}_{side}"), wheel_rpm);
        }
    }
}

/// The level of a definition over a vehicle's variables.
#[must_use]
pub fn level_of(def: &SoundDef, vars: &AiVariables) -> f32 {
    def.volcurves
        .iter()
        .map(|c| c.at(vars.get(&c.variable)))
        .product::<f32>()
        .clamp(0.0, 1.0)
}

/// The playback speed of a definition: a loop follows `variable / reference × scale`, a plain
/// sound its pitch factor.
#[must_use]
pub fn speed_of(def: &SoundDef, vars: &AiVariables) -> f32 {
    crate::sound_logic::speed(def, |n| vars.get(n)).max(crate::sound_logic::MIN_SPEED)
}

/// Level and speed of an AI loop: the shared rules (`1 + (variable / reference - 1) × scale`,
/// silent while the pitch is zero), and silent while its conditions do not hold.
pub(in crate::vehicle_sound) fn ai_loop(def: &SoundDef, vars: &AiVariables) -> (f32, f32) {
    if crate::sound_logic::conditions_hold(&def.conditions, |n| vars.get(n)) != Some(true) {
        return (0.0, 1.0);
    }
    crate::sound_logic::loop_voice(def, |n| vars.get(n))
}
