//! Vehicle scripts and key bindings.

use super::*;

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

/// Loads the vehicle's `.osc` scripts, variable lists and constants into a script engine.
/// A vehicle with `[scriptshare]` has none of its own and gets no engine. `part` is the unit's
/// number in the consist for a unit behind the head: its engine stands alone (no route, no
/// passenger counts) and is stepped by the head's system.
pub(super) fn attach_scripts(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    manifest: &BusManifest,
    bus_dir: &Path,
    seed: u64,
    part: Option<usize>,
) -> bool {
    if manifest.scripts.is_empty() {
        return false;
    }
    let seed = seed ^ (part.unwrap_or(0) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let (engine, diags) = opts_sim::script::load_vehicle_scripts(manifest, |rel| {
        let found = job.install.find_in(bus_dir, rel);
        if let Some(p) = &found {
            job.vehicle_dirs
                .push(opts_bevy::hot_reload::cache::normalize(p));
        }
        found
    });
    let errors = diags
        .iter()
        .filter(|d| d.severity == opts_sim::script::Severity::Error)
        .count();
    // Every compile problem once, with its file and line (the diagnostics of one compile are
    // distinct); a missing file is an error with the position tag, like other missing content.
    for d in &diags {
        if d.message == "file not found" {
            tracing::error!(at = %opts_bevy::position_tag(), file = %d.file, "vehicle script file not found");
        } else if d.severity == opts_sim::script::Severity::Error {
            tracing::error!(file = %d.file, line = d.line, "vehicle script: {}", d.message);
        } else {
            tracing::warn!(file = %d.file, line = d.line, "vehicle script: {}", d.message);
        }
    }
    tracing::info!(
        scripts = manifest.scripts.len(),
        diagnostics = diags.len(),
        errors,
        "vehicle scripts loaded"
    );
    let mut engine = engine;
    let map_name = job
        .manifest
        .friendly_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            job.config
                .map_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
        });
    let hof = opts_bevy::route::find_depot(bus_dir, job.install.root(), map_name);
    if let Some(h) = &hof {
        tracing::info!(
            depot_name = %h.name,
            termini = h.termini.len(),
            routes = h.routes.len(),
            "depot (.hof) file loaded for vehicle"
        );
    }
    // Nothing is selected until the driver (or the control protocol) selects a route; the
    // scripts' IBIS then asks the depot for it.
    let route = opts_bevy::route::RouteHandle::new(hof.map(std::sync::Arc::new));
    route.set_timetables(job.timetables());
    opts_bevy::route::apply_to_scripts(&mut engine, &route);

    engine.seed_random(seed as u32);
    let counts = opts_bevy::host_callbacks::PassengerCountsHandle::default();
    if part.is_none() {
        commands.entity(root).insert(counts.clone());
    }
    let ctx = opts_bevy::host_callbacks::VehicleHostContext {
        passengers: counts.0.clone(),
        seed,
        route: route.clone(),
        script_textures: job.vehicle_script_textures.clone(),
        fonts: std::sync::Arc::new(std::sync::RwLock::new(opts_bevy::text_texture::load_fonts(
            &job.install,
        ))),
        texture_base_dir: Some(bus_dir.join("Texture")),
        tickets: std::sync::Arc::new(crate::passengers::ticket_book(
            &super::passengers::ticket_pack(job),
        )),
        coins: std::sync::Arc::new(
            super::passengers::money_system(job)
                .map(|m| m.coins.iter().map(|c| c.value as f32).collect())
                .unwrap_or_default(),
        ),
        ..Default::default()
    };
    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
        ctx,
    ));
    // Equipment options of the chosen repaint (`[setvar]` of its `.cti` items).
    if let Some(r) = &job.repaint {
        let name = if job.config.livery.is_empty() {
            r.default_name.as_str()
        } else {
            job.config.livery.as_str()
        };
        for (var, value) in r.setvars.get(name).into_iter().flatten() {
            match value.trim().replace(',', ".").parse::<f32>() {
                Ok(v) => engine.set(var, v),
                Err(_) => tracing::warn!(var, value, "repaint [setvar] value is not a number"),
            }
        }
    }
    for (var, value) in &job.config.setvars {
        engine.set(var, *value as f32);
    }
    if part.is_some() {
        commands.entity(root).insert((
            opts_bevy::VehicleScripts::new(engine),
            opts_bevy::ScriptPart,
        ));
        return true;
    }
    commands.entity(root).insert((
        opts_bevy::VehicleScripts::new(engine),
        opts_bevy::ScriptStartup::default(),
        opts_bevy::route::VehicleRoute::new(route),
    ));
    true
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
    // The rebindable actions come from the settings (`crate::vehicle_keys`), which compose the
    // final `ScriptKeyBindings` from this base.
    commands.insert_resource(opts_bevy::ScriptKeyBase(bindings.0));
}
