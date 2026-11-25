//! Vehicle scripts and key bindings. Loading the scripts into an engine, with its host callbacks,
//! is in [`engine`].

use super::*;

mod engine;

pub(super) use engine::attach_scripts;

/// The seed of a vehicle's random numbers (`random`, `NrSpecRandom`): FNV-1a of its `.bus` path
/// below the installation and its session vehicle id, so the same vehicle in the same slot
/// draws the same numbers in every run and two vehicles differ (the entity bits it used to be
/// built from change with the order the world is spawned in).
pub(super) fn script_seed(
    root: &Path,
    bus_path: &Path,
    session_id: crate::session_vehicles::VehicleId,
) -> u64 {
    let rel = bus_path
        .strip_prefix(root)
        .unwrap_or(bus_path)
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    rel.bytes()
        .chain(std::iter::once(b'#'))
        .chain(session_id.to_le_bytes())
        .fold(0xCBF2_9CE4_8422_2325, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
        })
}

/// The cables a vehicle file declares, as the bridge reads them.
pub(super) fn cables_of(manifest: &BusManifest) -> Vec<opts_sim::script::consist::Cable> {
    manifest
        .extras
        .control_cables
        .iter()
        .map(|c| opts_sim::script::consist::Cable {
            front: c.front,
            kind: c.kind,
            index: c.index,
            receive: c.receive.clone(),
            send: c.send.clone(),
            coupling: c.coupling.clone(),
        })
        .collect()
}

/// Loads the installation's `Inputs/keyboard.cfg` into [`opts_bevy::ScriptKeyBase`], so the
/// player's keys raise the vehicle's script triggers. Without the file no key triggers exist.
pub(super) fn load_key_bindings(job: &WorldLoadJob, commands: &mut Commands) {
    let Some(path) = job.install.find_file("Inputs/keyboard.cfg") else {
        tracing::warn!("Inputs/keyboard.cfg not found; vehicle script triggers have no keys");
        return;
    };
    let config = match std::fs::read(&path) {
        Ok(bytes) => opts_core::formats::engine_config::KeyboardConfig::parse(&bytes),
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "cannot read keyboard.cfg: {e}");
            return;
        }
    };
    let bindings = opts_bevy::ScriptKeyBindings::from_config(&config);
    tracing::info!(
        entries = config.vehicle_bindings().count(),
        bound = bindings.0.len(),
        "vehicle script key bindings loaded"
    );
    // The rebindable actions come from the settings (`vehicle_keys` in `opts-game`), which
    // compose the final `ScriptKeyBindings` from this base.
    commands.insert_resource(opts_bevy::ScriptKeyBase(bindings.0));
}
