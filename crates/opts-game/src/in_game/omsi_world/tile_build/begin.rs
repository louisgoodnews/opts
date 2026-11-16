//! Starting a tile build: reads the tile, spawns its root and registers it as resident.

use super::*;

/// Starts the next queued tile: reads it, spawns its root and registers it as resident. `None`
/// when the queue holds nothing buildable (a tile that is resident already is skipped, one that
/// cannot be read counts as missing).
pub(in crate::in_game::omsi_world) fn begin(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
) -> Option<TileBuild> {
    let _t_total = opts_logging::stream_trace::span("tile.build");
    loop {
        let key = job.queue.pop_front()?;
        if job.resident.contains_key(&key) {
            continue;
        }
        let t_parse = opts_logging::stream_trace::span("tile.read_parse");
        let Some(tile) = job.read_tile(key) else {
            stats.tiles_missing += 1;
            continue;
        };
        job.tile_splines
            .insert(key, std::sync::Arc::new(tile.splines.clone()));
        // A tile rebuilt in place (Chrono switch) may still hold the collision of its previous
        // build.
        super::super::collision::unregister_tile(commands, key);
        for w in &tile.warnings {
            tracing::warn!(tile = ?key, "tile record left out: {w}");
        }
        drop(t_parse);
        opts_bevy::scene_trace::begin_tile(key);
        let origin = job.tile_offset(key);
        let tile_path = job.tile_path(key).unwrap_or_default();
        job.loading_deps.clear();
        job.loading_deps
            .insert(opts_bevy::hot_reload::cache::normalize(&tile_path));
        for overlay in job.chrono_overlays(key).to_vec() {
            job.loading_deps
                .insert(opts_bevy::hot_reload::cache::normalize(&overlay));
        }
        tracing::info!(
            tile = ?key,
            path = %tile_path.display(),
            objects = tile.objects.len(),
            splines = tile.splines.len(),
            "tile parsed"
        );
        let root = commands
            .spawn((
                Transform::default(),
                InGameEntity,
                TileRoot(key),
                Name::new(format!("Tile_{}_{}", key.0, key.1)),
            ))
            .id();
        job.resident.insert(key, root);
        return Some(TileBuild {
            key,
            tile,
            origin,
            root,
            children: Vec::new(),
            terrain: None,
            terrain_input: None,
            next_neighbour: 0,
            attached: Vec::new(),
            splines_failed_before: 0,
            phase: Phase::TerrainRead,
            next_object: 0,
        });
    }
}
