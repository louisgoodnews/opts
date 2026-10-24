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
    /// The playable definitions (at most [`MAX_AI_VOICES`]) with their files.
    pub voices: Vec<(Arc<SoundDef>, PathBuf)>,
    /// Definitions of the configuration that were left out (other variables, triggers,
    /// conditions, no file, over the cap).
    pub skipped: usize,
}

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

/// Whether the AI vehicle can play a definition: continuous, no condition, only supported
/// variables.
#[must_use]
pub fn playable(d: &SoundDef) -> bool {
    d.is_continuous()
        && d.conditions.is_empty()
        && variables_of(d).all(supported_variable)
        && (d.kind == SoundKind::Sound || d.pitch_variable.is_some())
}

/// Chooses the playable definitions of a configuration; `find` resolves a file name.
pub fn choose(cfg: &SoundConfig, find: impl Fn(&str) -> Option<PathBuf>) -> AiSoundSet {
    let mut set = AiSoundSet::default();
    for d in &cfg.sounds {
        if !playable(d) || set.voices.len() >= MAX_AI_VOICES {
            set.skipped += 1;
            continue;
        }
        match find(&d.file) {
            Some(path) => set.voices.push((Arc::new(d.clone()), path)),
            None => set.skipped += 1,
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
    (!set.voices.is_empty()).then(|| Arc::new(set))
}

/// One continuous sound of an AI vehicle, a child at its `[3d]` position.
#[derive(Component, Debug, Clone)]
pub struct AiVoice {
    /// The definition.
    pub def: Arc<SoundDef>,
    /// The sound file.
    pub file: PathBuf,
}

/// What the AI sounds did, for `audio.state`.
#[derive(Resource, Debug, Clone, Default)]
pub struct AiAudioStats {
    /// Voice entities of all AI vehicles.
    pub voices: usize,
    /// Voices with an emitter now.
    pub playing: usize,
}

/// Adds the voices of a vehicle type under `root`.
pub fn attach_ai(commands: &mut Commands, root: Entity, set: Option<&Arc<AiSoundSet>>) {
    let Some(set) = set else { return };
    commands.entity(root).with_children(|p| {
        for (def, file) in &set.voices {
            let at = def.position.unwrap_or([0.0; 3]);
            p.spawn((
                AiVoice {
                    def: def.clone(),
                    file: file.clone(),
                },
                Transform::from_translation(omsi_to_bevy_pos(Vec3::new(at[0], at[1], at[2]))),
                Name::new("AiVoice"),
            ));
        }
    });
}

/// Engine speed (rpm) of a road vehicle at `speed_kmh`: a gear model with idle 650 rpm, shifts
/// at 2,100 rpm and gear changes at 12, 25, 40, 60 and 90 km/h, so the pitch rises, drops at
/// every shift and rises again. ❓ OMSI's AI computes `AI_Motor` itself; its model is not known
/// (Q-AU05), this is a stand-in with the right shape.
#[must_use]
pub fn ai_engine_rpm(speed_kmh: f32) -> f32 {
    const IDLE: f32 = 650.0;
    const SHIFT: f32 = 2100.0;
    const GEARS: [f32; 6] = [0.0, 12.0, 25.0, 40.0, 60.0, 90.0];
    let v = speed_kmh.max(0.0);
    if v < 0.5 {
        return IDLE;
    }
    // Each gear covers from the previous change point (where it starts at 1,300 rpm) to its own
    // (2,100 rpm); the last one runs on at the same slope.
    let mut lo = 0.0;
    let mut hi = GEARS[1];
    for w in GEARS.windows(2) {
        lo = w[0];
        hi = w[1];
        if v < hi {
            break;
        }
    }
    let start = if lo == 0.0 { IDLE } else { 1300.0 };
    let f = ((v - lo) / (hi - lo)).max(0.0);
    (start + (SHIFT - start) * f).min(3200.0)
}

/// Writes the sound variables of one AI vehicle into its table.
pub fn publish_sound_variables(
    vars: &mut AiVariables,
    speed_ms: f32,
    accel_ms2: f32,
    brake: f32,
    wheel_radius: f32,
    wet: bool,
) {
    let kmh = speed_ms * 3.6;
    let throttle = if speed_ms > 0.3 {
        (accel_ms2 / 1.2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let rpm = ai_engine_rpm(kmh);
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
/// silent while the pitch is zero).
fn ai_loop(def: &SoundDef, vars: &AiVariables) -> (f32, f32) {
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
        let Ok(vars) = vehicles.get(parent.parent()) else {
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
                stream_ai_voices.after(crate::audio_content::poll_clip_loader),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(text: &str) -> SoundConfig {
        SoundConfig::parse(text)
    }

    #[test]
    fn only_variables_the_traffic_model_states_are_supported() {
        for ok in [
            "Velocity",
            "THROTTLE",
            "engine_n",
            "Wheel_RotationSpeed_1_L",
            "StreetCond",
        ] {
            assert!(supported_variable(ok), "{ok}");
        }
        for bad in [
            "door_1",
            "in_door_sound_01",
            "engine_turbo_RPM",
            "Wheel_RotationSpeed_1",
            "-1",
            "m_wheel",
        ] {
            assert!(!supported_variable(bad), "{bad}");
        }
    }

    #[test]
    fn a_configuration_is_cut_down_to_what_can_be_played() {
        let text = "[sound]\nrollen.wav\n3\n[3d]\n0\n0\n0\n5\n[volcurve]\nVelocity\n[pnt]\n0\n0\n[pnt]\n80\n1\n\
[loopsound]\nmotor.wav\n44100\nengine_n\n631\n1\n[3d]\n0\n2\n0.5\n8\n[volcurve]\nThrottle\n[pnt]\n0\n0.3\n[pnt]\n1\n1\n\
[sound]\ndoor.wav\n1\n[3d]\n0\n0\n0\n5\n[trigger]\nev_door\n\
[sound]\nhiss.wav\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nin_door_sound_01\n[pnt]\n0\n1\n\
[loopsound]\nturbo.wav\n44100\nengine_turbo_RPM\n1000\n1\n[3d]\n0\n0\n0\n5\n\
[sound]\ngone.wav\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nVelocity\n[pnt]\n0\n1\n";
        let set = choose(&cfg(text), |n| (n != "gone.wav").then(|| PathBuf::from(n)));
        let files: Vec<_> = set.voices.iter().map(|(d, _)| d.file.as_str()).collect();
        assert_eq!(files, ["rollen.wav", "motor.wav"]);
        assert_eq!(set.skipped, 4);
    }

    #[test]
    fn the_cap_keeps_the_first_voices() {
        let mut text = String::new();
        for i in 0..40 {
            text.push_str(&format!(
                "[sound]\nv{i}.wav\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nVelocity\n[pnt]\n0\n1\n"
            ));
        }
        let set = choose(&cfg(&text), |n| Some(PathBuf::from(n)));
        assert_eq!(set.voices.len(), MAX_AI_VOICES);
        assert_eq!(set.skipped, 40 - MAX_AI_VOICES);
    }

    #[test]
    fn the_engine_speed_rises_drops_at_a_shift_and_rises_again() {
        assert_eq!(ai_engine_rpm(0.0), 650.0);
        assert!(ai_engine_rpm(6.0) > 650.0 && ai_engine_rpm(11.9) < 2100.1);
        let before = ai_engine_rpm(11.99);
        let after = ai_engine_rpm(12.1);
        assert!(after < before - 500.0, "{before} -> {after}");
        assert!(ai_engine_rpm(20.0) > after);
        assert!(ai_engine_rpm(300.0) <= 3200.0);
        let mut last = 0.0;
        for k in 0..=120 {
            let v = ai_engine_rpm(k as f32);
            assert!((600.0..=3200.0).contains(&v));
            last = v;
        }
        assert!(last > 650.0);
    }

    #[test]
    fn variables_are_published_and_drive_level_and_pitch() {
        let mut vars = AiVariables::default();
        publish_sound_variables(&mut vars, 10.0, 0.6, 0.0, 0.5, false);
        assert!((vars.get("Velocity") - 36.0).abs() < 1e-4);
        assert!((vars.get("throttle") - 0.5).abs() < 1e-5);
        assert!(vars.get("wheel_rotationspeed_1_l") > 100.0);
        assert_eq!(vars.get("streetcond"), 0.0);
        let d = cfg("[loopsound]\nm.wav\n44100\nengine_n\n1000\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nVelocity\n[pnt]\n0\n0\n[pnt]\n72\n1\n")
            .sounds
            .remove(0);
        assert!((level_of(&d, &vars) - 0.5).abs() < 1e-4);
        let rpm = vars.get("engine_n");
        assert!((speed_of(&d, &vars) - rpm / 1000.0).abs() < 1e-4);
        // Standing: no throttle, idle.
        publish_sound_variables(&mut vars, 0.0, 1.0, 1.0, 0.5, true);
        assert_eq!(
            (
                vars.get("throttle"),
                vars.get("engine_n"),
                vars.get("streetcond")
            ),
            (0.0, 650.0, 1.0)
        );
    }
}

#[cfg(test)]
mod ai_name_tests {
    use super::is_ai_name;

    #[test]
    fn ai_variants_are_told_by_name() {
        for yes in [
            "Vehicles/MAN_NL_NG/MAN_GN92_main_KI.bus",
            "sound\\sound_GN92_M_ai.cfg",
            "ai_peugeot_106_S1.ovh",
            "KI_bus.bus",
            "Vehicles/x/AI_C2_e5g_3door.bus",
        ] {
            assert!(is_ai_name(yes), "{yes}");
        }
        for no in [
            "MAN_GN92_main.bus",
            "sound\\sound_GN92_M.cfg",
            "Kiel.bus",
            "maintenance.cfg",
            "Vehicles/Rain/x.bus",
        ] {
            assert!(!is_ai_name(no), "{no}");
        }
    }
}
