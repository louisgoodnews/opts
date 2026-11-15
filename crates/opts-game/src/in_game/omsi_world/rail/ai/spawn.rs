//! Spawning AI trains on the tracks and cyclists on the cycle paths around the camera.

use super::*;

fn next_rand(state: &mut u32) -> f32 {
    opts_sim::pedestrian::net::next_f32(state)
}

/// Spawns AI trains on tracks and cyclists on cycle paths around the camera.
pub fn spawn_ai_trains(
    time: Res<Time>,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    fleet: AiFleet,
    store: AiAssets,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    living: Query<&AiTrain>,
) {
    let AiFleet {
        settings,
        bike_settings,
        net,
        bike_net,
        mut pool,
        mut bike_pool,
    } = fleet;
    let AiAssets {
        mut meshes,
        mut materials,
        mut images,
    } = store;
    let (Some(mut job), Ok(cam)) = (job, camera.single()) else {
        return;
    };
    let centre = (cam.translation().x, -cam.translation().z);
    let trains = living
        .iter()
        .filter(|t| !t.bike && t.plan.is_none())
        .count();
    let riders = living.iter().filter(|t| t.bike).count();
    let dt = time.delta_secs();
    let mut assets = (&mut *meshes, &mut *materials, &mut *images);
    try_spawn(
        (
            false,
            dt,
            settings.max_trains.saturating_sub(trains),
            centre,
        ),
        &net.0,
        &mut pool,
        &mut job,
        &mut commands,
        &mut assets,
    );
    try_spawn(
        (
            true,
            dt,
            bike_settings.max_riders.saturating_sub(riders),
            centre,
        ),
        &bike_net.0,
        &mut bike_pool.0,
        &mut job,
        &mut commands,
        &mut assets,
    );
}

/// One spawn attempt per second and kind, when below the cap.
fn try_spawn(
    (bike, dt, room, centre): (bool, f32, usize, (f32, f32)),
    net: &PedestrianNet,
    pool: &mut RailAiPool,
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    assets: &mut SpawnAssets,
) {
    pool.elapsed += dt;
    if pool.elapsed < 1.0 || room == 0 || pool.specs.is_empty() || net.lane_count() == 0 {
        return;
    }
    pool.elapsed = 0.0;
    let mut rng = pool.rng;
    let spot = net.random_spot(centre, AI_SPAWN_MIN_M, AI_SPAWN_MAX_M, &mut rng);
    let Some((lane, s)) = spot else {
        pool.rng = rng;
        return;
    };
    let pick = (next_rand(&mut rng) * pool.specs.len() as f32) as usize % pool.specs.len();
    let dir = match net.lane(lane).map(|l| l.travel) {
        Some(Travel::Reverse) => -1,
        Some(Travel::Both) if next_rand(&mut rng) < 0.5 => -1,
        _ => 1,
    };
    pool.rng = rng;
    let spec = &pool.specs[pick];
    let Some((head, composition, _)) = build_train(spec, job, commands, assets, false) else {
        return;
    };
    tracing::info!(vehicle = %spec.name, units = spec.units.len(), lane, bike, "AI rail/cycle vehicle spawned");
    commands.entity(head).insert(AiTrain {
        bike,
        train: Train::new(lane, s, dir),
        speed: 0.0,
        composition,
        wheel_angle: 0.0,
        plan: None,
    });
}
