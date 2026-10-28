//! Stand-in gear model, AI variable publication, and 5 Hz voice streaming for AI traffic sounds.

use super::*;

/// The stand-in gear model of a vehicle class: idle and shift speed (rpm) and the speeds (km/h)
/// of the gear changes. A heavy vehicle idles at 650 rpm and shifts at 2,100 rpm at 12, 25, 40,
/// 60 and 90 km/h; a car idles at 800 rpm and shifts at 3,600 rpm at 18, 35, 55 and 85 km/h.
struct Gears {
    idle: f32,
    shift: f32,
    changes: &'static [f32],
}

const HEAVY: Gears = Gears {
    idle: 650.0,
    shift: 2100.0,
    changes: &[0.0, 12.0, 25.0, 40.0, 60.0, 90.0],
};
const LIGHT: Gears = Gears {
    idle: 800.0,
    shift: 3600.0,
    changes: &[0.0, 18.0, 35.0, 55.0, 85.0, 130.0],
};

/// Engine speed (rpm) of a road vehicle at `speed_kmh`: a gear model of its class, so the pitch
/// rises, drops at every shift and rises again. ❓ OMSI's AI computes `AI_Motor` itself; its model
/// is not known (Q-AU05), this is a stand-in with the right shape.
#[must_use]
pub fn ai_engine_rpm(speed_kmh: f32, heavy: bool) -> f32 {
    let g = if heavy { &HEAVY } else { &LIGHT };
    let v = speed_kmh.max(0.0);
    if v < 0.5 {
        return g.idle;
    }
    // Each gear covers from the previous change point (where it starts at 60 % of the shift
    // speed) to its own (the shift speed); the last one runs on at the same slope.
    let mut lo = 0.0;
    let mut hi = g.changes[1];
    for w in g.changes.windows(2) {
        lo = w[0];
        hi = w[1];
        if v < hi {
            break;
        }
    }
    let start = if lo == 0.0 { g.idle } else { g.shift * 0.62 };
    let f = ((v - lo) / (hi - lo)).max(0.0);
    (start + (g.shift - start) * f).min(g.shift * 1.5)
}

/// The gear of the stand-in model at `speed_kmh` (`0` below walking pace).
#[must_use]
pub fn ai_gear(speed_kmh: f32, heavy: bool) -> f32 {
    let g = if heavy { &HEAVY } else { &LIGHT };
    let past = g.changes[1..].iter().filter(|c| speed_kmh >= **c).count();
    if speed_kmh < 0.5 {
        0.0
    } else {
        (past + 1) as f32
    }
}

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
pub(super) fn ai_loop(def: &SoundDef, vars: &AiVariables) -> (f32, f32) {
    if crate::sound_logic::conditions_hold(&def.conditions, |n| vars.get(n)) != Some(true) {
        return (0.0, 1.0);
    }
    crate::sound_logic::loop_voice(def, |n| vars.get(n))
}

/// Starts, updates and stops the emitters of AI voices five times a second.
#[allow(clippy::too_many_arguments)]
pub fn stream_ai_voices(
    time: Res<Time>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    mut voices: Query<(
        Entity,
        &AiVoice,
        &GlobalTransform,
        &ChildOf,
        Option<&mut SoundEmitter>,
    )>,
    vehicles: Query<&AiVariables>,
    mut loader: ResMut<ClipLoader>,
    mut stats: ResMut<AiAudioStats>,
    mut since: Local<f32>,
    mut commands: Commands,
) {
    *since += time.delta_secs();
    if *since < 0.2 {
        return;
    }
    *since = 0.0;
    let Some(listener) = listener.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let (mut total, mut playing) = (0, 0);
    for (entity, voice, tf, parent, emitter) in &mut voices {
        total += 1;
        let Ok(vars) = vehicles.get(voice.owner.unwrap_or_else(|| parent.parent())) else {
            continue;
        };
        let curve = SceneryVoice {
            def: voice.def.clone(),
            file: None,
            playback: crate::sound_logic::Playback::Loop,
            was_active: false,
        }
        .distance();
        let d = listener.distance(tf.translation());
        match emitter {
            Some(mut e) => {
                if d > curve.max * 1.25 {
                    commands.entity(entity).remove::<SoundEmitter>();
                } else {
                    playing += 1;
                    let (gain, speed) = ai_loop(&voice.def, vars);
                    if (e.gain - gain).abs() > 1e-3 {
                        e.gain = gain;
                    }
                    if (e.speed - speed).abs() > 1e-3 {
                        e.speed = speed;
                    }
                }
            }
            None if d <= curve.max => {
                let (gain, speed) = ai_loop(&voice.def, vars);
                if gain > 0.0 {
                    if let ClipState::Ready(clip) = loader.state(&voice.file) {
                        commands.entity(entity).insert(
                            SoundEmitter::new(clip, SoundBus::Vehicles)
                                .looping()
                                .spatial(curve)
                                .with_speed(speed)
                                .with_gain(gain),
                        );
                    }
                }
            }
            None => {}
        }
    }
    stats.voices = total;
    stats.playing = playing;
}

/// Registers the AI sound systems.
#[derive(Default)]
pub struct AiAudioPlugin;

impl Plugin for AiAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClipLoader>()
            .init_resource::<AiAudioStats>()
            .add_systems(
                Update,
                (stream_ai_voices, fire_ai_events).after(crate::audio_content::poll_clip_loader),
            );
    }
}
