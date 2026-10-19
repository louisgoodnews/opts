//! Streaming the tiles of the world: loading the next tile, advancing a tile build and streaming around the player.

use super::*;

/// Loads the next queued tile completely: terrain, splines and scenery objects. The loading
/// screen's and the benchmark's way; streaming in the game uses [`advance_tile_loading`] with a
/// frame budget instead.
pub fn load_next_tile(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    ground_materials: Option<&mut Assets<GroundMaterial>>,
) {
    let mut budget = FrameBudget::new(std::time::Duration::MAX);
    advance_tile_loading(
        job,
        stats,
        commands,
        meshes,
        materials,
        images,
        ground_materials,
        &mut budget,
        1,
    );
}

/// Works on tile builds until `budget` is used up or `max_tiles` tiles are finished: continues the
/// build that is in progress, else starts the next queued tile. Returns the number of tiles
/// finished. With an unlimited budget this is [`load_next_tile`].
#[allow(clippy::too_many_arguments)]
pub fn advance_tile_loading(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    mut ground_materials: Option<&mut Assets<GroundMaterial>>,
    budget: &mut FrameBudget,
    max_tiles: usize,
) -> usize {
    let mut finished = 0;
    while finished < max_tiles {
        if job.building.is_none() {
            if job.queue.is_empty() || !budget.allow() {
                break;
            }
            job.building = tile_build::begin(job, stats, commands);
            if job.building.is_none() {
                break;
            }
        }
        let done = tile_build::advance(
            job,
            stats,
            commands,
            meshes,
            materials,
            images,
            ground_materials.as_deref_mut(),
            budget,
        );
        if !done {
            break;
        }
        finished += 1;
    }
    finished
}

/// Marks the root entity of a streamed tile.
#[derive(Component, Debug, Clone, Copy)]
pub struct TileRoot(pub (i32, i32));

/// Keeps the tiles around the camera loaded and unloads distant ones (one tile load per frame).
///
/// The observer is the 3D camera. In radius mode the tiles within the radius (Chebyshev
/// distance) are resident and tiles beyond the radius + 1 are despawned (hysteresis avoids
/// thrashing at borders); in *load all* mode every tile is wanted and none is despawned
/// ([`tile_loading`], M-0080). Parsed objects and textures stay cached, so revisiting a tile is
/// cheap.
#[allow(clippy::too_many_arguments)]
pub fn stream_world_tiles(
    job: Option<ResMut<WorldLoadJob>>,
    state: Res<crate::in_game::types::SimulationLoadState>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut stats: ResMut<WorldStats>,
    mut commands: Commands,
    mut assets: LoadAssets,
) {
    let Some(mut job) = job else { return };
    if !state.is_complete {
        return;
    }
    let Ok(cam) = camera.single() else { return };
    let p = cam.translation();
    let centre = job.grid.key_at(Vec2::new(p.x, -p.z), job.start_tile);
    let mode = job.tile_mode();
    let far = tile_loading::evict(mode, centre, job.resident.keys().copied());
    let _t_evict = opts_logging::stream_trace::span("stream.evict");
    for key in far {
        if let Some(unload) = job.unload_tile(key, false) {
            unload.apply(&mut commands);
        }
    }
    drop(_t_evict);
    stats.tiles_loaded = job.resident.len();

    if job.queue.is_empty() && job.building.is_none() {
        let wanted = tile_loading::wanted(mode, centre, job.listed_tiles(), |k| {
            job.resident.contains_key(&k) || job.missing_tiles.contains(&k)
        });
        // The nearest few, so that a frame with budget left can go on with the next one.
        let mut queued = 0;
        for key in wanted {
            if queued == REFILL {
                break;
            }
            if job.tile_available(key) {
                job.queue.push_back(key);
                queued += 1;
            }
        }
    }
    if job.has_tile_work() {
        let (meshes, materials, images) = (
            &mut assets.meshes,
            &mut assets.materials,
            &mut assets.images,
        );
        // A budget of zero is the old behaviour: one whole tile per frame.
        let (mut budget, max_tiles) = if job.stream_budget.is_zero() {
            (FrameBudget::new(std::time::Duration::MAX), 1)
        } else {
            (FrameBudget::new(job.stream_budget), usize::MAX)
        };
        advance_tile_loading(
            &mut job,
            &mut stats,
            &mut commands,
            meshes,
            materials,
            images,
            assets.ground_materials.as_deref_mut(),
            &mut budget,
            max_tiles,
        );
    }
}
