//! Starting a world load: the install and manifest of the session, then the job itself.

use super::*;

/// Parses `global.cfg`, picks the start tile and queues its 3×3 neighbourhood (the base map,
/// without Chrono).
pub fn begin_world_load(config: OmsiSessionConfig) -> Result<WorldLoadJob, String> {
    begin_world_load_at(config, None)
}

/// The installation with the catalogue's lookups, when the session has them.
fn install_of(config: &OmsiSessionConfig) -> OmsiInstall {
    let mut install = OmsiInstall::new(&config.root);
    if let Some(l) = &config.lookup {
        install = install.with_lookup(l.clone());
    }
    if let Some(l) = &config.name_lookup {
        install = install.with_name_lookup(l.clone());
    }
    if let Some(l) = &config.relocate_lookup {
        install = install.with_relocate_lookup(l.clone());
    }
    if let Some(l) = &config.twin_lookup {
        install = install.with_twin_lookup(l.clone());
    }
    install
}

/// Reads and parses the map's `global.cfg`.
fn read_manifest(install: &OmsiInstall, config: &OmsiSessionConfig) -> Result<MapManifest, String> {
    let cfg_path = install
        .find_in(&config.map_dir, "global.cfg")
        .ok_or_else(|| format!("global.cfg not found in {}", config.map_dir.display()))?;
    let bytes = std::fs::read(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let manifest =
        MapManifest::parse(&bytes).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    tracing::info!(
        path = %cfg_path.display(),
        name = %manifest.name,
        friendly = ?manifest.friendly_name,
        tiles = manifest.tiles.len(),
        entry_points = manifest.entry_points.len(),
        "global.cfg parsed"
    );
    Ok(manifest)
}

/// [`begin_world_load`] for a map date: the Chrono entries active on it are applied to every
/// tile read (`None`: the base map).
pub fn begin_world_load_at(
    config: OmsiSessionConfig,
    map_date: Option<opts_core::formats::chrono::ChronoDate>,
) -> Result<WorldLoadJob, String> {
    let install = install_of(&config);
    let manifest = read_manifest(&install, &config)?;

    let tile_files: HashMap<(i32, i32), String> = manifest
        .tiles
        .iter()
        .map(|t| ((t.x, t.y), t.file_name.clone()))
        .collect();

    let mut job = new_job(install, config, manifest, tile_files);

    opts_bevy::scene_trace::begin_session(&job.map_name(), job.install.root());
    job.init_chrono(map_date);

    measure_grid(&mut job);

    choose_start(&mut job)?;
    let start_tile = job.start_tile;

    for dy in -1..=1 {
        for dx in -1..=1 {
            let key = (start_tile.0 + dx, start_tile.1 + dy);
            if job.tile_path(key).is_some() {
                job.queue.push_back(key);
            } else {
                if job.manifest.tiles.iter().any(|t| (t.x, t.y) == key) {
                    job.missing_listed += 1;
                }
                tracing::debug!(tile = ?key, "neighbour tile not present in map");
            }
        }
    }
    job.total_tiles = job.queue.len();
    tracing::info!(
        tiles = job.total_tiles,
        "tiles discovered for the 3x3 neighbourhood"
    );
    Ok(job)
}
