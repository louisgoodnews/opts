//! The inputs the scripts read: the axle states, the host inputs from the simulation and clock, and the weather.

use super::*;

/// Wheel speed in rpm for a road speed (m/s) and a wheel radius (m).
fn rpm_of_omega(omega_rad_s: f32) -> f32 {
    omega_rad_s * 60.0 / std::f32::consts::TAU
}

/// The axle states the scripts read (one entry per axle of the simulation).
fn collect_axle_inputs(sim: &opts_sim::VehicleSimulation) -> Vec<AxleInput> {
    let wheels = sim.body.as_ref().map_or(&[][..], |b| &b.wheels[..]);
    let travel = |k: usize| wheels.get(k).map_or(0.0, |w| w.omsi_travel_m());
    let rest = sim.body.as_ref().map_or(0.0, |b| b.rest_travel_m());
    let mut axles: Vec<AxleInput> = sim
        .axles
        .iter()
        .enumerate()
        .map(|(i, axle)| {
            let wheel = |w: &opts_sim::WheelState, side: usize| WheelInput {
                speed_rpm: rpm_of_omega(w.angular_velocity_rad_s),
                rotation_rad: w.rotation_rad,
                steering_rad: w.steer_angle_rad,
                suspension_m: travel(i * 2 + side),
            };
            AxleInput {
                left: wheel(&axle.left_wheel, 0),
                right: wheel(&axle.right_wheel, 1),
            }
        })
        .collect();

    let r0 = sim
        .axles
        .first()
        .map_or(0.49, |a| a.left_wheel.rolling_radius_m)
        .max(0.1);
    let rolling = WheelInput {
        speed_rpm: rpm_of_omega(sim.chassis.forward_speed_m_s() / r0),
        rotation_rad: sim.axles.first().map_or(0.0, |a| a.left_wheel.rotation_rad),
        steering_rad: 0.0,
        suspension_m: rest,
    };
    for unit in &sim.trailer_loads {
        for _ in &unit.axles {
            axles.push(AxleInput {
                left: rolling,
                right: rolling,
            });
        }
    }
    axles
}

fn apply_clock_inputs(inputs: &mut HostInputs, c: &crate::scenery_runtime::SceneryClock) {
    inputs.envir_brightness = c.envir_brightness();
    inputs.temperature_c = c.temperature_c;
    inputs.abs_hum = c.weather.abs_hum;
    inputs.precip_type = c.weather.precip_type;
    inputs.precip_rate = c.weather.precip_rate;
    inputs.sun_alt_deg = c.sun_alt_deg;
    inputs.time_of_day_s = (c.seconds_of_day % 86_400.0) as f32;
    (inputs.year, inputs.month, inputs.day) = (c.year, c.month, c.day);
}

/// The host's state a vehicle's scripts may read, from the simulation and the session's clock
/// (the table of `opts_sim::script::inputs`, units of the OMSI wiki as far as known: `Velocity`
/// in km/h ❓, pedals `0..1`).
#[must_use]
pub fn host_inputs(
    controller: &VehicleController,
    clock: Option<&crate::scenery_runtime::SceneryClock>,
) -> HostInputs {
    let sim = &controller.sim;
    let mut inputs = HostInputs {
        velocity_km_h: sim.chassis.forward_speed_km_h(),
        velocity_ground_km_h: sim.chassis.forward_speed_km_h(),
        a_trans: [
            sim.chassis.local_acceleration.x,
            sim.chassis.local_acceleration.y,
            sim.chassis.ground_vertical_acc_m_s2
                + sim.body.as_ref().map_or(0.0, |b| b.vertical_acc_m_s2),
        ],
        // The fuel model's level (M-0123); a vehicle without a model reads full.
        tank_percent: sim.fuel_level().map_or(100.0, |level| level * 100.0),
        throttle: sim.inputs.throttle(),
        brake: sim.inputs.service_brake(),
        ai: false,
        ai_engine: 0.0,
        ..HostInputs::default()
    };
    inputs.n_wheel_rpm = if sim.axles.is_empty() {
        0.0
    } else {
        sim.axles
            .iter()
            .map(|a| {
                rpm_of_omega(
                    0.5 * (a.left_wheel.angular_velocity_rad_s
                        + a.right_wheel.angular_velocity_rad_s),
                )
            })
            .sum::<f32>()
            / sim.axles.len() as f32
    };
    inputs.axles = collect_axle_inputs(sim);
    if let Some(c) = clock {
        apply_clock_inputs(&mut inputs, c);
    }
    inputs
}

/// Gives a vehicle's scripts the weather: the system values `Weather_Temperature`,
/// `Weather_AbsHum`, `PrecipType` and `PrecipRate`, and the two precipitation values as local
/// variables as well (the scripts of the corpus read them both ways).
pub fn feed_weather(
    engine: &mut ScriptEngine,
    temperature_c: f32,
    vars: &crate::weather::WeatherVars,
) {
    engine.set_system("Weather_Temperature", temperature_c);
    engine.set_system("Weather_AbsHum", vars.abs_hum);
    engine.set_system("PrecipType", vars.precip_type);
    engine.set_system("PrecipRate", vars.precip_rate);
    engine.set("preciptype", vars.precip_type);
    engine.set("preciprate", vars.precip_rate);
}
