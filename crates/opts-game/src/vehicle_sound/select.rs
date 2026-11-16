//! Choosing the sound definitions an AI vehicle plays: which variables it can state, which definitions qualify, and the set built from a configuration.

use super::*;

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
