//! The script engine of a scheduled vehicle unit: its scripts, depot and timetable callbacks.

use super::*;

/// The script engine and route of one unit of a scheduled vehicle (a road vehicle's head, a unit
/// of a train), `None` when the unit has no scripts or the run is unknown to its depot.
pub(in crate) fn engine_for(
    job: &mut WorldLoadJob,
    bus_path: &Path,
    canvases: Arc<RwLock<Vec<CpuScriptTexture>>>,
    run: Option<&RunRef>,
    seed: u64,
    setvars: &[(String, String)],
) -> Option<(VehicleScripts, VehicleRoute)> {
    let manifest = std::fs::read(bus_path)
        .ok()
        .and_then(|b| BusManifest::parse_bytes(&b).ok())
        .filter(|m| !m.scripts.is_empty())?;
    let dir = bus_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let (mut engine, _) = opts_sim::script::load_vehicle_scripts(&manifest, |rel| {
        crate::script_file::read_script(job.install.find_in(&dir, rel))
    });
    let folder = job
        .config
        .map_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let friendly = job.manifest.friendly_name.as_deref().unwrap_or("");
    let hof = opts_bevy::route::find_depot(&dir, job.install.root(), &[friendly, folder]);
    let route = RouteHandle::new(hof.map(Arc::new));
    route.set_timetables(job.timetables());
    if let Some(run) = run {
        if route.select_run(&run.line_file, run.tour, run.run).is_err() {
            return None;
        }
    }
    opts_bevy::route::apply_to_scripts(&mut engine, &route);
    engine.seed_random(seed as u32);
    if let Some(n) =
        super::super::super::fleet_number::fleet_number(&job.install, &dir, &manifest, seed)
    {
        engine.set_string("number", &n);
    }
    engine.set("ai", 1.0);
    engine.set("ai_engine", 1.0);
    // The equipment of the chosen repaint (`[setvar]` of its `.cti`): the model's parts are shown
    // or hidden by these variables, so without them a body can be missing from the vehicle.
    for (var, value) in setvars {
        if let Ok(v) = value.trim().replace(',', ".").parse::<f32>() {
            engine.set(var, v);
        }
    }
    let ctx = opts_bevy::host_callbacks::VehicleHostContext {
        seed,
        route: route.clone(),
        script_textures: canvases,
        fonts: Arc::new(RwLock::new(opts_bevy::text_texture::load_fonts(
            &job.install,
        ))),
        font_bitmaps: opts_bevy::script_text::FontBitmapSource::new(&job.install),
        texture_base_dir: Some(dir.join("Texture")),
        ..Default::default()
    };
    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
        ctx,
    ));
    let mut vehicle_route = VehicleRoute::new(route);
    vehicle_route.pending_target_s = TARGET_OFFER_S;
    Some((VehicleScripts::new(engine), vehicle_route))
}
