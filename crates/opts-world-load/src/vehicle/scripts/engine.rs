//! Loading a vehicle's scripts into a script engine: the diagnostics, the host callbacks and the
//! repaint's equipment options, then the engine's components on the vehicle's root entity.

use super::super::*;

/// Logs every compile problem once, with its file and line (the diagnostics of one compile are
/// distinct); a missing file is an error with the position tag, like other missing content.
fn log_diagnostics(manifest: &BusManifest, diags: &[opts_sim::script::Diagnostic]) {
    let errors = diags
        .iter()
        .filter(|d| d.severity == opts_sim::script::Severity::Error)
        .count();
    for d in diags {
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
}

/// What the host callbacks of the vehicle's scripts need: passenger counts, tickets, coins, fonts
/// and the texture folder.
fn host_context(
    job: &WorldLoadJob,
    (bus_dir, seed): (&Path, u64),
    route: &opts_bevy::route::RouteHandle,
    counts: &opts_bevy::host_callbacks::PassengerCountsHandle,
) -> opts_bevy::host_callbacks::VehicleHostContext {
    opts_bevy::host_callbacks::VehicleHostContext {
        passengers: counts.0.clone(),
        seed,
        route: route.clone(),
        script_textures: job.vehicle_script_textures.clone(),
        fonts: std::sync::Arc::new(std::sync::RwLock::new(opts_bevy::text_texture::load_fonts(
            &job.install,
        ))),
        font_bitmaps: opts_bevy::script_text::FontBitmapSource::new(&job.install),
        texture_base_dir: Some(bus_dir.join("Texture")),
        tickets: std::sync::Arc::new(opts_passenger_spec::ticket_book(
            &super::super::passengers::ticket_pack(job),
        )),
        coins: std::sync::Arc::new(
            super::super::passengers::money_system(job)
                .map(|m| m.coins.iter().map(|c| c.value as f32).collect())
                .unwrap_or_default(),
        ),
        ..Default::default()
    }
}

/// The equipment options of the chosen repaint (`[setvar]` of its `.cti` items).
fn apply_repaint_setvars(engine: &mut opts_sim::script::ScriptEngine, job: &WorldLoadJob) {
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
}

/// Loads the vehicle's `.osc` scripts, variable lists and constants into a script engine.
/// A vehicle with `[scriptshare]` has none of its own and gets no engine. `part` is the unit's
/// number in the consist for a unit behind the head: its engine stands alone (no route, no
/// passenger counts) and is stepped by the head's system.
pub(in super::super) fn attach_scripts(
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
    log_diagnostics(manifest, &diags);
    let mut engine = engine;
    let folder = job
        .config
        .map_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let friendly = job.manifest.friendly_name.as_deref().unwrap_or("");
    let hof = opts_bevy::route::find_depot(bus_dir, job.install.root(), &[friendly, folder]);
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
    if let Some(n) =
        super::super::super::fleet_number::fleet_number(&job.install, bus_dir, manifest, seed)
    {
        engine.set_string("number", &n);
    }
    let counts = opts_bevy::host_callbacks::PassengerCountsHandle::default();
    if part.is_none() {
        commands.entity(root).insert(counts.clone());
    }
    let ctx = host_context(job, (bus_dir, seed), &route, &counts);
    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
        ctx,
    ));
    apply_repaint_setvars(&mut engine, job);
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
