//! Building the load job: its state, the grid of the map and the start tile.

use super::*;

/// A job with nothing loaded yet.
pub(super) fn new_job(
    install: OmsiInstall,
    config: OmsiSessionConfig,
    manifest: MapManifest,
    tile_files: HashMap<(i32, i32), String>,
) -> WorldLoadJob {
    WorldLoadJob {
        install,
        cache: ContentCache::default(),
        config,
        manifest,
        tile_files,
        queue: VecDeque::new(),
        tile_water: HashMap::new(),
        water_material: None,
        water_enabled: true,
        resident: HashMap::new(),
        tile_mode: opts_tile_loading::TileMode::default(),
        missing_tiles: HashSet::new(),
        building: None,
        stream_budget: opts_tile_loading::DEFAULT_STREAM_BUDGET,
        stream_mode: Default::default(),
        stream_focus: None,
        stream_status: Default::default(),
        total_tiles: 0,
        sli_cache: HashMap::new(),
        tile_deps: HashMap::new(),
        loading_deps: HashSet::new(),
        vehicle_dirs: Vec::new(),
        spline_index: None,
        tile_splines: HashMap::new(),
        tile_aligned: HashMap::new(),
        placed: HashMap::new(),
        tile_ground: None,
        sco_cache: HashMap::new(),
        sound_cfgs: HashMap::new(),
        sound_hooks: std::sync::Arc::new(crate::sound_hooks::NoSoundHooks),
        crossing_bases: HashMap::new(),
        ground_faces: HashMap::new(),
        tile_objects: HashMap::new(),
        tile_surfaces: HashMap::new(),
        timetables: None,
        collision_bounds: HashMap::new(),
        scenery: super::super::scenery_objects::SceneryObjectCache::new(),
        parklists: HashMap::new(),
        chrono: super::super::chrono::ChronoContext::default(),
        spline_materials: HashMap::new(),
        surf_strips: HashMap::new(),
        surf_maps: HashMap::new(),
        plate_tris: HashMap::new(),
        fonts: opts_bevy::text_texture::FontRegistry::default(),
        vehicle_text_defs: Vec::new(),
        vehicle_script_defs: Vec::new(),
        vehicle_script_textures: Arc::default(),
        ground_material: None,
        ground_array: None,
        spawn: None,
        repaint: None,
        model_repaints: HashMap::new(),
        formation_cars: Vec::new(),
        start_tile: (0, 0),
        grid: opts_core::formats::map::TileGrid::default(),
        missing_listed: 0,
        camera: None,
        camera_std_index: 0,
        no_vehicle: false,
        parked_permille: 1000,
        resolved_entry: None,
        start_entry: None,
    }
}

/// Tile grid: 300 m, except for maps with world coordinates (Mercator rows when the splines'
/// border chains agree with them, else the measured pitch).
pub(super) fn measure_grid(job: &mut WorldLoadJob) {
    let mut sample = Vec::new();
    let keys: Vec<(i32, i32)> = job.manifest.tiles.iter().map(|t| (t.x, t.y)).collect();
    for key in keys {
        if let Some(tile) = job.read_tile(key) {
            sample.push((key, tile.splines));
        }
        if sample.len() >= 80 {
            break;
        }
    }
    let (grid, fit) =
        opts_core::formats::map::TileGrid::measure(&sample, job.manifest.world_coordinates);
    job.grid = grid;
    tracing::info!(
        ?grid,
        pairs = fit.pairs,
        median_step_m = ?fit.median_step,
        mercator_residual_m = ?fit.mercator_residual,
        sampled_tiles = sample.len(),
        "tile grid"
    );
}

/// Start tile and spawn: a chosen trip's first bus stop, else the chosen `[entrypoints]` record if
/// it resolves to a placed `entrypoint_*.sco` object, otherwise the first road spline of the first
/// usable tile. Sets the job's spawn point and start tile (the floating origin: all scene
/// coordinates are relative to the start tile's south-west corner).
pub(super) fn choose_start(job: &mut WorldLoadJob) -> Result<(), String> {
    let mut keys: Vec<(i32, i32)> = job.manifest.tiles.iter().map(|t| (t.x, t.y)).collect();
    keys.dedup();
    let at_stop = job
        .config
        .start_station
        .and_then(|id| job.find_stop_spawn(id, &keys));
    if job.config.start_station.is_some() && at_stop.is_none() {
        tracing::warn!(
            station = ?job.config.start_station,
            "the trip's first stop is not on the map; starting at the entry point"
        );
    }
    let mut start = at_stop.or_else(|| job.find_entry_spawn(&keys));
    job.start_entry = job.resolved_entry.take();
    if start.is_none() {
        tracing::warn!(
            entry = %job.config.entry_point,
            "entry point not resolvable; falling back to the first road spline"
        );
        for key in keys {
            let Some(tile) = job.read_tile(key) else {
                continue;
            };
            if let Some(spawn) = job.find_drivable_spawn(&tile) {
                start = Some((key, spawn));
                break;
            }
        }
    }
    let (start_tile, spawn) = start.ok_or("no entry point or drivable road spline found")?;
    tracing::info!(
        tile = ?start_tile,
        spawn_omsi = ?spawn.0,
        heading_deg = spawn.1,
        "start tile and spawn point selected"
    );

    // Floating origin: all scene coordinates are relative to the start tile's south-west
    // corner. Real maps use world coordinates in the millions of metres (Berlin-Spandau:
    // 3.4e6 north), where f32 resolution is a quarter of a metre. The spawn point is already in
    // the start tile's frame.
    job.spawn = Some(spawn);
    job.start_tile = start_tile;
    Ok(())
}
