//! Sounds of the player's vehicle (M-0102): the `[sound]` configuration of the `.bus` becomes
//! [`PlayerVoice`] children of the vehicle, driven by the vehicle's script variables and events.
//!
//! * a voice that is a loop while active (`engine_n` pitch, `velocity` rolling noise, the
//!   compressor, wipers, sirens …) has a looping emitter near the listener, its level from the
//!   `[volcurve]`s, its pitch from the loop's variable, silenced while its `[conditionSingle]`s
//!   do not hold;
//! * a voice that fires once (`[trigger]` on a raised script event, or `[noloop]` with a condition
//!   that turns true) plays a one-shot at its `[3d]` place (doors, switches, kneeling, starter);
//! * `[viewpoint]` 2 plays outside the vehicle only, 5 inside only.
//!
//! The rules of when a definition plays are in [`crate::sound_logic`]. Not yet: trailers and the
//! articulated bus' second unit, announcements (`(T.F)` payloads), the `Snd_*` mixer inputs,
//! force feedback, the horn and other definitions with a condition mode that is not understood
//! (counted in [`PlayerAudioStats::unsupported`]).

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::audio::{Distance, SoundBus, SoundEmitter};
use opts_bevy::{omsi_to_bevy_pos, MainCamera, VehicleScripts};
use opts_core::formats::sound::SoundDef;
use opts_core::formats::vehicles::BusManifest;

use crate::audio_content::{ClipLoader, ClipState};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::sound_logic::{
    conditions_hold, level, loop_voice, playback_of, viewpoint_allows, Playback,
};

/// One sound definition of the player's vehicle, a child at its `[3d]` position.
#[derive(Component, Debug, Clone)]
pub struct PlayerVoice {
    /// The definition.
    pub def: Arc<SoundDef>,
    /// The sound file.
    pub file: PathBuf,
    /// What it does when active.
    pub playback: Playback,
    /// The conditions held at the last look (edge detection of `[noloop]` voices).
    pub was_active: bool,
    /// The vehicle whose script variables and events drive this voice when it hangs on another
    /// unit of it (a trailer shares the head's scripts, `[scriptshare]`); `None`: its parent.
    pub owner: Option<Entity>,
    /// Where the emitter's gain and speed are heading (set twenty times a second, followed every
    /// frame by [`smooth_player_voices`]): a change of the engine speed is a glide, not steps.
    pub goal: Option<(f32, f32)>,
    /// Seconds the voice has been active without a break: the volume curves with the variable
    /// `-1` ([`crate::sound_logic::AGE_VARIABLE`]) fade the voice in over it.
    pub active_s: f32,
}

impl PlayerVoice {
    /// The fade with distance: full inside the `[3d]` range, silent beyond six times it (at least
    /// 40 m more). Cabin sounds have ranges of a metre or two and are heard around the vehicle.
    #[must_use]
    pub fn distance(&self) -> Distance {
        let r = self.def.range.max(0.5);
        Distance::new(r, (r * 6.0).max(r + 40.0))
    }
}

/// What the definitions of a vehicle's configuration came to (on the vehicle root).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlayerSoundInfo {
    /// Definitions in the configuration.
    pub total: usize,
    /// Left out: a condition mode that is not understood, or `[noloop]` alone.
    pub unsupported: usize,
    /// Left out: the sound file was not found.
    pub unresolved: usize,
}

/// What the player's sounds did, for `audio.state`.
#[derive(Resource, Debug, Clone, Default)]
pub struct PlayerAudioStats {
    /// Voice entities.
    pub voices: usize,
    /// Loops with an emitter now (the listener is within their range).
    pub loops: usize,
    /// One-shots started.
    pub one_shots: u64,
    /// Triggers that found their file still decoding.
    pub cold_misses: u64,
    /// Definitions left out for an unsupported condition mode.
    pub unsupported: usize,
    /// Definitions whose file was not found.
    pub unresolved: usize,
    /// The listener is inside the vehicle.
    pub inside: bool,
    /// The last script events the sound layer saw and how many one-shot voices each started (a
    /// name with 0 has no voice in the vehicle's configuration, or none whose conditions held).
    pub events: std::collections::VecDeque<(String, usize)>,
}

/// Adds the voices of the vehicle's `[sound]` configuration under `root`.
pub fn attach(
    job: &WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    manifest: &BusManifest,
    bus_dir: &std::path::Path,
) {
    attach_unit(job, commands, root, None, manifest, bus_dir);
}

/// Adds the voices of the `[sound]` configuration of one unit of a vehicle under `unit`. A trailer
/// (`owner` is the head) has its own configuration at its own places but the head's variables and
/// events: the engine of an articulated bus can sit in its second unit (the Bremen MAN NG313's
/// engine loops are in `sound_NG313_T.cfg`).
pub fn attach_unit(
    job: &WorldLoadJob,
    commands: &mut Commands,
    unit: Entity,
    owner: Option<Entity>,
    manifest: &BusManifest,
    bus_dir: &std::path::Path,
) {
    let root = unit;
    let Some(rel) = manifest.sound_cfg.as_deref() else {
        return;
    };
    let Some((cfg, cfg_dir)) = job.read_sound_config(bus_dir, rel) else {
        tracing::warn!(cfg = rel, "the vehicle's sound configuration was not found");
        return;
    };
    let mut info = PlayerSoundInfo {
        total: cfg.sounds.len(),
        ..Default::default()
    };
    let mut voices = Vec::new();
    for def in &cfg.sounds {
        let playback = playback_of(def);
        if playback == Playback::Unsupported {
            info.unsupported += 1;
            continue;
        }
        let Some(file) = job.find_sound_file(&cfg_dir, &def.file) else {
            info.unresolved += 1;
            continue;
        };
        let at = def.position.unwrap_or([0.0; 3]);
        voices.push((
            PlayerVoice {
                def: Arc::new(def.clone()),
                file,
                playback,
                was_active: false,
                owner,
                goal: None,
                active_s: 0.0,
            },
            omsi_to_bevy_pos(Vec3::new(at[0], at[1], at[2])),
        ));
    }
    tracing::info!(
        total = info.total,
        voices = voices.len(),
        unsupported = info.unsupported,
        unresolved = info.unresolved,
        "player vehicle sounds"
    );
    commands.entity(root).insert(info).with_children(|p| {
        for (voice, pos) in voices {
            p.spawn((
                voice,
                Transform::from_translation(pos),
                Name::new("PlayerVoice"),
            ));
        }
    });
}

/// Whether `listener` (world position) is inside the vehicle at `vehicle`: a box about the body
/// of a solo bus (1.7 m either side, up to 3.6 m high, 8 m either way; ❓ the model's own bounds
/// would be better).
#[must_use]
pub fn listener_inside(vehicle: &GlobalTransform, listener: Vec3) -> bool {
    inside_box(vehicle, listener)
}

/// Whether the listener is in the cabin: the driver's and the passengers' views are, the external
/// view is not (its camera hangs a few metres behind the vehicle, still within a body-sized box);
/// the walking and the free camera are where their position says.
#[must_use]
pub fn listener_in_cabin(
    mode: Option<opts_bevy::CameraMode>,
    vehicle: &GlobalTransform,
    listener: Vec3,
) -> bool {
    match mode {
        Some(opts_bevy::CameraMode::Cockpit | opts_bevy::CameraMode::Passenger) => true,
        Some(opts_bevy::CameraMode::External) => false,
        _ => inside_box(vehicle, listener),
    }
}

fn inside_box(vehicle: &GlobalTransform, listener: Vec3) -> bool {
    let p = vehicle.affine().inverse().transform_point3(listener);
    p.x.abs() < 1.7 && p.y > -0.3 && p.y < 3.6 && p.z.abs() < 8.0
}

type VoiceQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut PlayerVoice,
        &'static GlobalTransform,
        &'static ChildOf,
        Option<&'static mut SoundEmitter>,
    ),
>;

/// Reads a vehicle variable by name (case-insensitive).
fn variable(scripts: &VehicleScripts, name: &str) -> f32 {
    scripts
        .engine
        .lock()
        .map_or(0.0, |e| e.get(&name.to_lowercase()))
}

/// Updates the loops and the `[noloop]` edges twenty times a second.
#[allow(clippy::too_many_arguments)]
pub fn stream_player_voices(
    time: Res<Time>,
    camera: Option<Res<opts_bevy::CameraState>>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    mut voices: VoiceQuery,
    vehicles: Query<(&VehicleScripts, &GlobalTransform)>,
    mut loader: ResMut<ClipLoader>,
    mut stats: ResMut<PlayerAudioStats>,
    mut since: Local<f32>,
    mut commands: Commands,
) {
    *since += time.delta_secs();
    if *since < 0.05 {
        return;
    }
    let dt = std::mem::take(&mut *since);
    let Some(listener) = listener.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let (mut total, mut loops, mut inside_any) = (0, 0, false);
    for (entity, mut voice, tf, parent, emitter) in &mut voices {
        total += 1;
        let Ok((scripts, vehicle_tf)) =
            vehicles.get(voice.owner.unwrap_or_else(|| parent.parent()))
        else {
            continue;
        };
        let inside = listener_in_cabin(camera.as_ref().map(|c| c.mode), vehicle_tf, listener);
        inside_any |= inside;
        let Some(active) = conditions_hold(&voice.def.conditions, |n: &str| variable(scripts, n))
        else {
            continue;
        };
        let audible_here = viewpoint_allows(voice.def.viewpoint, inside);
        // How long it has been sounding: the volume curves of the starter and retarder loops fade in
        // over it (`[volcurve] -1`).
        if active && audible_here {
            voice.active_s += dt;
        } else {
            voice.active_s = 0.0;
        }
        let age = voice.active_s;
        let read = |n: &str| {
            if n == crate::sound_logic::AGE_VARIABLE {
                age
            } else {
                variable(scripts, n)
            }
        };
        match voice.playback {
            Playback::Loop => {
                let curve = voice.distance();
                let d = listener.distance(tf.translation());
                match emitter {
                    Some(e) => {
                        if d > curve.max * 1.25 {
                            commands.entity(entity).remove::<SoundEmitter>();
                            continue;
                        }
                        loops += 1;
                        let (gain, spd) = if active && audible_here {
                            loop_voice(&voice.def, read)
                        } else {
                            (0.0, e.speed)
                        };
                        // The emitter follows these every frame (`smooth_player_voices`).
                        voice.goal = Some((gain, spd));
                    }
                    None if d <= curve.max && active && audible_here => {
                        let (gain, spd) = loop_voice(&voice.def, read);
                        // A loop that is silent now gets its emitter when it sounds.
                        if gain > 0.0 {
                            if let ClipState::Ready(clip) = loader.state(&voice.file) {
                                commands.entity(entity).insert(
                                    SoundEmitter::new(clip, SoundBus::Vehicles)
                                        .looping()
                                        .spatial(curve)
                                        .with_speed(spd)
                                        .with_gain(gain),
                                );
                            }
                        }
                    }
                    None => {}
                }
            }
            Playback::OnEdge => {
                let edge = active && !voice.was_active;
                if voice.was_active != active {
                    voice.was_active = active;
                }
                if edge && audible_here {
                    let level = level(&voice.def, read);
                    fire(
                        &mut commands,
                        &mut loader,
                        &mut stats,
                        entity,
                        &voice,
                        level,
                    );
                }
            }
            Playback::OnTrigger | Playback::Unsupported => {}
        }
    }
    stats.voices = total;
    stats.loops = loops;
    stats.inside = inside_any;
}

/// Starts decoding the files of the one-shots (doors, switches, brakes, the horn's start and end)
/// as soon as the voices exist: the first ask of a clip that is still undecoded is silent
/// (`ClipLoader`), and a door that opens for the first time must be heard.
pub fn preload_one_shots(
    voices: Query<&PlayerVoice, Added<PlayerVoice>>,
    mut loader: ResMut<ClipLoader>,
) {
    for voice in &voices {
        if matches!(voice.playback, Playback::OnTrigger | Playback::OnEdge) {
            let _ = loader.state(&voice.file);
        }
    }
}

/// How fast a loop's gain and speed follow their goals (seconds to cover about two thirds of the
/// way): short enough to follow the engine, long enough to hide the 20 Hz updates.
const GLIDE_S: f32 = 0.08;

/// Moves the gain and speed of the voices' emitters towards their goals every frame.
pub fn smooth_player_voices(
    time: Res<Time>,
    mut voices: Query<(&mut PlayerVoice, &mut SoundEmitter)>,
) {
    let k = 1.0 - (-time.delta_secs() / GLIDE_S).exp();
    for (voice, mut emitter) in &mut voices {
        let Some((gain, speed)) = voice.goal else {
            continue;
        };
        let step = |now: f32, goal: f32| {
            if (goal - now).abs() < 1e-3 {
                goal
            } else {
                now + (goal - now) * k
            }
        };
        let (g, s) = (step(emitter.gain, gain), step(emitter.speed, speed));
        if (emitter.gain - g).abs() > f32::EPSILON {
            emitter.gain = g;
        }
        if (emitter.speed - s).abs() > f32::EPSILON {
            emitter.speed = s;
        }
    }
}

/// Starts a one-shot of `voice`: a child emitter at its place, gone when it has played.
fn fire(
    commands: &mut Commands,
    loader: &mut ClipLoader,
    stats: &mut PlayerAudioStats,
    entity: Entity,
    voice: &PlayerVoice,
    level: f32,
) {
    match loader.state(&voice.file) {
        ClipState::Ready(clip) => {
            stats.one_shots += 1;
            let emitter = SoundEmitter::new(clip, SoundBus::Effects)
                .spatial(voice.distance())
                .with_speed(voice.def.pitch.clamp(0.3, 3.5))
                .with_gain(level);
            commands.entity(entity).with_children(|p| {
                p.spawn((emitter, Transform::IDENTITY, Name::new("player one-shot")));
            });
        }
        // The first ask only starts the decode; the next one is heard.
        ClipState::Loading => stats.cold_misses += 1,
        ClipState::Failed => {}
    }
}

/// Plays the one-shots whose `[trigger]` the vehicle's scripts raised this frame.
pub fn fire_player_triggers(
    camera: Option<Res<opts_bevy::CameraState>>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    mut vehicles: Query<(Entity, &mut VehicleScripts, &GlobalTransform)>,
    voices: Query<(Entity, &PlayerVoice, &ChildOf)>,
    mut loader: ResMut<ClipLoader>,
    mut stats: ResMut<PlayerAudioStats>,
    mut commands: Commands,
) {
    let listener = listener.iter().next().map(GlobalTransform::translation);
    for (vehicle, mut scripts, vehicle_tf) in &mut vehicles {
        let raised = {
            let s = scripts.bypass_change_detection();
            if s.raised.is_empty() {
                continue;
            }
            s.take_raised()
        };
        let inside = listener
            .is_some_and(|l| listener_in_cabin(camera.as_ref().map(|c| c.mode), vehicle_tf, l));
        for name in raised {
            let mut started = 0;
            // The voices of the vehicle: on its root, and on the units behind it that share its
            // scripts.
            for (child, voice, parent) in &voices {
                if voice.owner.unwrap_or_else(|| parent.parent()) != vehicle {
                    continue;
                }
                if voice.playback != Playback::OnTrigger
                    || !voice
                        .def
                        .triggers
                        .iter()
                        .any(|t| t.trim().eq_ignore_ascii_case(&name))
                    || !viewpoint_allows(voice.def.viewpoint, inside)
                {
                    continue;
                }
                let read = |n: &str| variable(&scripts, n);
                if conditions_hold(&voice.def.conditions, read) != Some(true) {
                    continue;
                }
                let lvl = level(&voice.def, read);
                fire(&mut commands, &mut loader, &mut stats, child, voice, lvl);
                started += 1;
            }
            if stats.events.len() >= 64 {
                stats.events.pop_front();
            }
            stats.events.push_back((name, started));
        }
    }
}

/// Registers the player vehicle sound systems.
#[derive(Default)]
pub struct PlayerAudioPlugin;

impl Plugin for PlayerAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClipLoader>()
            .init_resource::<PlayerAudioStats>()
            .add_systems(
                Update,
                (
                    stream_player_voices,
                    fire_player_triggers,
                    smooth_player_voices.after(stream_player_voices),
                    preload_one_shots,
                )
                    .after(crate::audio_content::poll_clip_loader),
            );
    }
}

#[cfg(test)]
mod tests;
