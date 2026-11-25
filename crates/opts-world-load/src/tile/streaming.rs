//! Streaming the tiles of the world: loading the next tile, advancing a tile build and streaming around the player.

use super::*;

mod hybrid;

use hybrid::{evictions, plan_frame, refill, HybridState};

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
        (job, stats, commands),
        (meshes, materials, images),
        ground_materials,
        (&mut budget, 1),
    );
}

/// Works on tile builds until `budget` is used up or `max_tiles` tiles are finished: continues the
/// build that is in progress, else starts the next queued tile. Returns the number of tiles
/// finished. With an unlimited budget this is [`load_next_tile`].
pub fn advance_tile_loading(
    (job, stats, commands): (&mut WorldLoadJob, &mut WorldStats, &mut Commands),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    mut ground_materials: Option<&mut Assets<GroundMaterial>>,
    (budget, max_tiles): (&mut FrameBudget, usize),
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
            (job, stats, commands),
            (meshes, materials, images),
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

/// The camera and the vehicle the tile streamer follows, and the frame time (grouped to stay
/// within Bevy's parameter limit).
#[derive(bevy::ecs::system::SystemParam)]
pub struct StreamViewpoint<'w, 's> {
    /// The 3D camera (the observer).
    camera: Query<'w, 's, &'static GlobalTransform, With<opts_bevy::MainCamera>>,
    /// The player's vehicle (the track), when there is one.
    vehicle: Query<'w, 's, &'static GlobalTransform, With<opts_bevy::PlayerVehicle>>,
    /// The frame time.
    time: Res<'w, Time>,
}

/// Keeps the tiles around the camera loaded and unloads distant ones (one tile load per frame).
///
/// The observer is the 3D camera. In radius mode the tiles within the radius (Chebyshev
/// distance) are resident and tiles beyond the radius + the hysteresis are despawned; in *load
/// all* mode every tile is wanted and none is despawned ([`opts_tile_loading`], M-0080). The speed of
/// the player picks the regime (M-0114, `hybrid`): at speed the queue is ordered by the heading
/// and tiles are loaded ahead; when slow the tile being built places its nearest objects first.
/// Parsed objects and textures stay cached, so revisiting a tile is cheap.
pub fn stream_world_tiles(
    job: Option<ResMut<WorldLoadJob>>,
    state: Res<opts_game_state::in_game::types::SimulationLoadState>,
    viewpoint: StreamViewpoint,
    mut hybrid: Local<HybridState>,
    mut stats: ResMut<WorldStats>,
    mut commands: Commands,
    mut assets: LoadAssets,
) {
    let Some(mut job) = job else { return };
    if !state.is_complete {
        return;
    }
    let Ok(cam) = viewpoint.camera.single() else {
        return;
    };
    let p = cam.translation();
    let centre = job.grid.key_at(Vec2::new(p.x, -p.z), job.start_tile);
    // The track is the vehicle (the camera when there is none); the observer stays the camera.
    let track = viewpoint
        .vehicle
        .single()
        .map_or((p.x, -p.z), |v| (v.translation().x, -v.translation().z));
    let plan = plan_frame(&mut job, &mut hybrid, track, viewpoint.time.delta_secs());
    let _t_evict = opts_logging::stream_trace::span("stream.evict");
    for key in evictions(&job, centre, &plan) {
        if let Some(unload) = job.unload_tile(key, false) {
            unload.apply(&mut commands);
        }
    }
    drop(_t_evict);
    stats.tiles_loaded = job.resident.len();

    if job.queue.is_empty() && job.building.is_none() {
        refill(&mut job, centre, track, &plan);
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
            (&mut job, &mut stats, &mut commands),
            (meshes, materials, images),
            assets.ground_materials.as_deref_mut(),
            (&mut budget, max_tiles),
        );
    }
}
