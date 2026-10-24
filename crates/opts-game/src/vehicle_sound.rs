//! Sounds of AI traffic (M-0140): the vehicle's AI sound configuration (`[sound_ai]` of a `.bus`;
//! for cars the shared `Sounds/AI_Cars` file their `[sound]` names) becomes [`AiVoice`] children of
//! the vehicle, and a system plays the continuous ones (engine, rolling noise, wheels) near the
//! listener with levels and pitch from the vehicle's [`AiVariables`].
//!
//! AI vehicles run without their scripts, so only the variables the traffic model can state are
//! offered (`velocity`, `throttle`, `brake`, `engine_n` from a gear model ❓, the wheel speeds,
//! `streetcond`, …; see [`supported_variable`]). A definition that reads any other variable, needs
//! a trigger or a condition, is left out when the vehicle type loads and counted in
//! [`AiSoundSet::skipped`]: of the 45,043 definitions in the 499 AI configurations of the
//! installation, 6,319 qualify (median 8 per configuration).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::audio::{SoundBus, SoundEmitter};
use opts_bevy::{omsi_to_bevy_pos, AiVariables, MainCamera};
use opts_core::formats::sound::{SoundConfig, SoundDef, SoundKind};
use opts_core::formats::vehicles::BusManifest;

use crate::audio_content::{ClipLoader, ClipState};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::scenery_sound::SceneryVoice;

/// Most continuous voices one AI vehicle carries.
pub const MAX_AI_VOICES: usize = 16;

/// The definitions of an AI vehicle type that can be played, with their sound files.
#[derive(Debug, Clone, Default)]
pub struct AiSoundSet {
    /// The loops the traffic model can drive (at most [`MAX_AI_VOICES`]) with their files:
    /// continuous, their conditions and variables all stated by the model.
    pub voices: Vec<(Arc<SoundDef>, PathBuf)>,
    /// One-shots the traffic model raises itself (the horn, [`HOST_EVENTS`]).
    pub events: Vec<(Arc<SoundDef>, PathBuf)>,
    /// Definitions that need the vehicle's own scripts (door triggers and door variables,
    /// anything else the model does not state): played only by a vehicle that runs scripts, a
    /// scheduled bus (at most [`MAX_SCRIPTED_VOICES`]).
    pub scripted: Vec<(Arc<SoundDef>, PathBuf)>,
    /// Definitions of the configuration that were left out (a condition mode that is not
    /// understood, interior-only sounds, no file, over a cap).
    pub skipped: usize,
}

/// Most script-driven voices a scheduled AI vehicle carries.
pub const MAX_SCRIPTED_VOICES: usize = 48;
/// The events the traffic model raises for an AI vehicle (lower case): `ev_AI_Horn`.
pub const HOST_EVENTS: [&str; 1] = ["ev_ai_horn"];

/// Whether the traffic model states a variable: the speed and pedals, the engine speed from the
/// gear model, the wheel speeds, the road state.
#[must_use]
pub fn supported_variable(name: &str) -> bool {
    let n = name.trim().to_lowercase();
    matches!(
        n.as_str(),
        "velocity"
            | "velocity_ground"
            | "throttle"
            | "brake"
            | "engine_n"
            | "ai_engine"
            | "ai_motor"
            | "streetcond"
            | "engine_throttle_injection"
            | "n_wheel"
            | "antrieb_n_kardanwelle"
            // The state of the drive: running, not starting, going forward in its gear, wipers
            // with rain, the electrics on (see `publish_sound_variables`).
            | "engine_on"
            | "engine_starter"
            | "antrieb_wendesatz"
            | "antrieb_getr_aktugang"
            | "wiperrunning"
            | "elec_busbar_main_sw"
    ) || n
        .strip_prefix("wheel_rotationspeed_")
        .is_some_and(|rest| matches!(rest.as_bytes(), [a, b'_', s] if a.is_ascii_digit() && matches!(s, b'l' | b'r')))
}

/// The variables a definition reads: the pitch variable of a loop and its volume curves.
fn variables_of(d: &SoundDef) -> impl Iterator<Item = &str> {
    d.pitch_variable
        .as_deref()
        .into_iter()
        .chain(d.volcurves.iter().map(|c| c.variable.as_str()))
}

/// The variables a definition's conditions read.
fn condition_variables(d: &SoundDef) -> impl Iterator<Item = &str> {
    d.conditions.iter().map(|c| c.variable.as_str())
}

/// Whether the traffic model can drive a definition as a loop: it loops while its conditions
/// hold, and everything it reads (pitch, volume curves, conditions) is a variable the model
/// states.
#[must_use]
pub fn playable(d: &SoundDef) -> bool {
    crate::sound_logic::playback_of(d) == crate::sound_logic::Playback::Loop
        && variables_of(d).all(supported_variable)
        && condition_variables(d).all(supported_variable)
        && (d.kind == SoundKind::Sound || d.pitch_variable.is_some())
}

/// Whether a definition is a one-shot on an event the traffic model raises.
#[must_use]
pub fn host_event(d: &SoundDef) -> bool {
    crate::sound_logic::playback_of(d) == crate::sound_logic::Playback::OnTrigger
        && !d.triggers.is_empty()
        && d.triggers
            .iter()
            .all(|t| HOST_EVENTS.contains(&t.trim().to_ascii_lowercase().as_str()))
        && condition_variables(d).all(supported_variable)
        && variables_of(d).all(supported_variable)
}

/// Sorts the definitions of a configuration: the loops the traffic model drives, the one-shots
/// it raises, and those that need the vehicle's scripts; `find` resolves a file name. Sounds
/// that play only inside the vehicle (`[viewpoint]` 2 and 6) are left out: AI vehicles are
/// heard from outside.
pub fn choose(cfg: &SoundConfig, find: impl Fn(&str) -> Option<PathBuf>) -> AiSoundSet {
    let mut set = AiSoundSet::default();
    for d in &cfg.sounds {
        let playback = crate::sound_logic::playback_of(d);
        if playback == crate::sound_logic::Playback::Unsupported
            || !crate::sound_logic::viewpoint_allows(d.viewpoint, false)
        {
            set.skipped += 1;
            continue;
        }
        let Some(path) = find(&d.file) else {
            set.skipped += 1;
            continue;
        };
        let def = Arc::new(d.clone());
        if playable(d) {
            if set.voices.len() < MAX_AI_VOICES {
                set.voices.push((def, path));
            } else {
                set.skipped += 1;
            }
        } else if host_event(d) {
            set.events.push((def, path));
        } else if set.scripted.len() < MAX_SCRIPTED_VOICES {
            set.scripted.push((def, path));
        } else {
            set.skipped += 1;
        }
    }
    set
}

/// Whether a file name marks an AI variant of a vehicle (`MAN_GN92_main_KI.bus`,
/// `ai_peugeot_106.bus`, `sound_GN92_M_ai.cfg`): a `_ki` or `_ai` part, or a `ki_`/`ai_` start.
#[must_use]
pub fn is_ai_name(file: &str) -> bool {
    let stem = file
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(file)
        .rsplit_once('.')
        .map_or(file, |(s, _)| s)
        .to_lowercase();
    stem.starts_with("ki_")
        || stem.starts_with("ai_")
        || stem
            .split(['_', '-', ' '])
            .any(|part| part == "ki" || part == "ai")
}

/// The sound set of a vehicle file: its `[sound_ai]` configuration, else its `[sound]` when that
/// is one of the shared `Sounds/AI_Cars` files or when the vehicle file (`bus`) or the
/// configuration is an AI variant by name (`..._KI.bus` names its own light `..._ai.cfg`). A bus
/// whose `[sound]` is the heavy player configuration and which has no `[sound_ai]` stays silent.
#[must_use]
pub fn ai_sound_set(
    job: &WorldLoadJob,
    manifest: &BusManifest,
    dir: &Path,
    bus: &Path,
) -> Option<Arc<AiSoundSet>> {
    let rel = manifest.sound_ai_cfg.as_deref().or_else(|| {
        manifest.sound_cfg.as_deref().filter(|s| {
            s.replace('\\', "/")
                .to_lowercase()
                .contains("sounds/ai_cars")
                || is_ai_name(s)
                || is_ai_name(&bus.to_string_lossy())
        })
    })?;
    let (cfg, cfg_dir) = job.read_sound_config(dir, rel)?;
    let set = choose(&cfg, |name| job.find_sound_file(&cfg_dir, name));
    (!(set.voices.is_empty() && set.events.is_empty() && set.scripted.is_empty()))
        .then(|| Arc::new(set))
}

/// One continuous sound of an AI vehicle, a child at its `[3d]` position.
#[derive(Component, Debug, Clone)]
pub struct AiVoice {
    /// The definition.
    pub def: Arc<SoundDef>,
    /// The sound file.
    pub file: PathBuf,
    /// The head of the vehicle whose variables drive this voice when it hangs on a following
    /// unit (the second unit of an articulated bus has its own configuration at its own places
    /// and the head's variables); `None`: its parent.
    pub owner: Option<Entity>,
}

mod events;
pub use events::{fire_ai_events, AiEventVoice, AiEvents};

/// What the AI sounds did, for `audio.state`.
#[derive(Resource, Debug, Clone, Default)]
pub struct AiAudioStats {
    /// Voice entities of all AI vehicles.
    pub voices: usize,
    /// Voices with an emitter now.
    pub playing: usize,
    /// One-shots started by events the traffic model raised (the horn).
    pub events: u64,
}

/// Adds the voices of a vehicle type under `root`.
pub fn attach_ai(commands: &mut Commands, root: Entity, set: Option<&Arc<AiSoundSet>>) {
    let Some(set) = set else { return };
    attach_ai_unit(commands, root, None, set);
    commands
        .entity(root)
        .insert(AiEvents::default())
        .with_children(|p| {
            for (def, file) in &set.events {
                let at = def.position.unwrap_or([0.0; 3]);
                p.spawn((
                    AiEventVoice {
                        def: def.clone(),
                        file: file.clone(),
                    },
                    Transform::from_translation(omsi_to_bevy_pos(Vec3::new(at[0], at[1], at[2]))),
                    Name::new("AiEventVoice"),
                ));
            }
        });
}

/// Adds the loops of one unit's sound set under `unit`: for a following unit of an articulated
/// bus `owner` is the head, whose variables drive them.
pub fn attach_ai_unit(
    commands: &mut Commands,
    unit: Entity,
    owner: Option<Entity>,
    set: &Arc<AiSoundSet>,
) {
    commands.entity(unit).with_children(|p| {
        for (def, file) in &set.voices {
            let at = def.position.unwrap_or([0.0; 3]);
            p.spawn((
                AiVoice {
                    def: def.clone(),
                    file: file.clone(),
                    owner,
                },
                Transform::from_translation(omsi_to_bevy_pos(Vec3::new(at[0], at[1], at[2]))),
                Name::new("AiVoice"),
            ));
        }
    });
}

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
fn ai_loop(def: &SoundDef, vars: &AiVariables) -> (f32, f32) {
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

#[cfg(test)]
mod tests;
