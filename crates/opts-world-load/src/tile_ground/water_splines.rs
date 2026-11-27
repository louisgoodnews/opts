//! Water and spline mesh stages of a tile build.

use opts_bevy::{build_water_surface, water_material, WaterWaves};
use opts_core::formats::map::{MapTile, MapWater};

use super::*;

mod splines;
pub(in super::super) use splines::spawn_splines;

/// Water stage: the tile's `.map.water` surface over the terrain it floods.
///
/// Drawn when the tile announces water (`[water]` marker) and the file is there: 93 of 1,906
/// water files in the installation lack the marker, scattered over 17 maps, and are taken as
/// water the editor switched off (`docs/src/formats/map/tiles.md`, Q-M07).
pub(in super::super) fn spawn_water(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
    (tile, terrain, key, origin): (&MapTile, Option<&TileTerrain>, (i32, i32), (f32, f32)),
    children: &mut Vec<Entity>,
) {
    if !tile.has_water || !job.water_enabled {
        return;
    }
    let rel = format!("{}.water", job.tile_files[&key]);
    let Some(path) = job.install.find_in(&job.config.map_dir, &rel) else {
        tracing::warn!(tile = ?key, "tile announces water but has no .water file");
        return;
    };
    job.loading_deps
        .insert(opts_bevy::hot_reload::cache::normalize(&path));
    let water = match std::fs::read(&path).map(|b| MapWater::parse(&b)) {
        Ok(Ok(w)) => w,
        Ok(Err(e)) => {
            tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "water parse failed: {e}");
            return;
        }
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "water read failed: {e}");
            return;
        }
    };
    let Some(terrain) = terrain else {
        tracing::warn!(tile = ?key, "water without a terrain grid is not drawn");
        return;
    };
    let Some(surface) = build_water_surface(&terrain.grid, &water, origin, terrain.cell) else {
        tracing::debug!(tile = ?key, levels = ?water.levels, "water lies below the terrain everywhere");
        return;
    };
    let material = job
        .water_material
        .get_or_insert_with(|| materials.add(water_material()))
        .clone();
    stats.scene_vertices += surface.mesh.count_vertices();
    stats.water_tiles += 1;
    job.tile_water.insert(key, water);
    let mesh = meshes.add(surface.mesh);
    let waves = WaterWaves {
        mesh: mesh.clone(),
        rest: surface.rest,
        depths: surface.depths,
    };
    children.push(
        commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                waves,
                Name::new(format!("Water_{}_{}", key.0, key.1)),
            ))
            .id(),
    );
}
