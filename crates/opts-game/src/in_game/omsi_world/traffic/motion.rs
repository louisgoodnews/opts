//! Moving traffic vehicles and their script variables.

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

/// A scheduled vehicle follows its run farther than ordinary traffic stays (m), so a bus the
/// player watched drive off does not vanish at the edge of the traffic disc.
const SCHEDULED_DESPAWN_M: f32 = 700.0;
/// A scheduled vehicle is removed this long (s) after arriving at the last stop of its run ...
const RETIRE_AFTER_S: f64 = 8.0;
/// ... if it is at least this far (m) from the camera; nearer, it drives on and leaves like any
/// other vehicle.
const RETIRE_FAR_M: f64 = 80.0;

/// Steps the agents, places the vehicle models and removes finished or distant ones.
pub fn move_traffic(
    time: Res<Time>,
    mut commands: Commands,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    mut state: ResMut<TrafficState>,
    mut sched: ResMut<ScheduledTraffic>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut vehicles: Query<(
        Entity,
        &mut TrafficVehicle,
        &mut Transform,
        &mut Visibility,
        Option<&TrafficChain>,
    )>,
) {
    let dt = f64::from(time.delta_secs().min(0.2));
    // The lights follow the session's time of day.
    if let Some(e) = &env {
        state.world.set_clock(e.seconds_of_day());
    }
    // Sub-steps of at most 0.1 s keep the car-following law stable.
    let steps = (dt / 0.1).ceil().max(1.0);
    for _ in 0..steps as usize {
        state.world.step(dt / steps);
    }
    for e in state.world.drain_events() {
        if let opts_traffic::TrafficEvent::Stuck { id, lane } = e {
            tracing::warn!(id = id.0, lane = lane.0, "AI vehicle stuck, despawned");
        }
        sched.on_event(&e);
        // A scheduled vehicle at the last stop of its run is removed shortly after, when nobody
        // can see it go (see `RETIRE_FAR_M`); in view it drives on as ordinary traffic.
        if let opts_traffic::TrafficEvent::ArrivedAtStop { id, clock, .. } = e {
            if state.world.agent(id).is_some_and(|a| a.stops.len() == 1) {
                sched.finish_at(id, clock, RETIRE_AFTER_S);
            }
        }
    }
    let now = state.world.clock();
    for id in sched.take_retired(now) {
        let near = state
            .world
            .agent(id)
            .and_then(|a| a.pose(state.world.graph()))
            .is_some_and(|(p, _)| {
                camera.single().is_ok_and(|c| {
                    let c = c.translation();
                    DVec2::new(f64::from(c.x), -f64::from(c.z)).distance(p) < RETIRE_FAR_M
                })
            });
        if !near {
            state.world.despawn(id);
        }
    }
    if std::env::var_os("OPTS_DBG_OBSTACLES").is_some() {
        tracing::info!(
            obstacles = state.world.obstacles().len(),
            lanes_blocked = state.world.phantom_count(),
            agents = state.world.agents().len(),
            lanes = state.world.graph().len(),
            "traffic obstacles"
        );
        for a in state.world.agents() {
            let Some((pos, h)) = a.pose(state.world.graph()) else {
                continue;
            };
            for o in state.world.obstacles() {
                let d = pos.distance(o.centre);
                if d < 25.0 {
                    tracing::info!(
                        id = a.id.0,
                        lane = a.lane.0,
                        s = a.s,
                        speed = a.speed,
                        accel = a.accel,
                        waiting = a.waiting_s,
                        ignore_until = a.ignore_conflicts_until,
                        dist = d,
                        heading = h,
                        agent_at = ?pos,
                        bus_at = ?o.centre,
                        bus_len = o.length,
                        "agent near obstacle"
                    );
                }
            }
        }
    }
    let cam = camera.single().ok().map(GlobalTransform::translation);
    let mut far = Vec::new();
    for (entity, mut v, mut transform, mut visibility, chain) in &mut vehicles {
        let graph = state.world.graph();
        let pose = state.world.agent(v.id).and_then(|a| {
            let (_, h) = a.pose(graph)?;
            let (rear_z, pitch) = a.ground_pose(graph)?;
            Some((a.rear_axle, h, rear_z, pitch))
        });
        let Some((rear, heading, z, pitch)) = pose else {
            despawn_chain(&mut commands, entity, chain);
            continue;
        };
        // The body lies along the axles; the model origin sits `rear_axle_y` from the rear axle.
        let dir = DVec2::new(heading.sin(), heading.cos());
        let origin = rear - dir * f64::from(v.rear_axle_y);
        let height = z as f32 + v.delta_height;
        let bevy = omsi_to_bevy_pos(Vec3::new(origin.x as f32, origin.y as f32, height));
        let limit = if sched.run_of(v.id).is_some() {
            SCHEDULED_DESPAWN_M
        } else {
            DESPAWN_M
        };
        if cam.is_some_and(|c| c.distance(bevy) > limit) {
            far.push(v.id);
            despawn_chain(&mut commands, entity, chain);
            continue;
        }
        (v.origin, v.heading, v.height) = (origin, heading, height);
        transform.translation = bevy;
        transform.rotation = omsi_euler_to_bevy_quat(
            -(heading.to_degrees() as f32),
            pitch.to_degrees() as f32,
            0.0,
        );
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }
    for id in far {
        state.world.despawn(id);
    }
}
