//! The pedestrian spawner: the density-derived target, the character and walking speed drawn for
//! each new walker, and the spot on a lane around the camera.

use super::super::*;

use super::walker::spawn_walker;

/// Factor of `global.cfg` `[trafficdensity_passenger]` (hour/value pairs, linear between them,
/// the hour wrapping at midnight) at `hour`; `1` when the map gives no curve.
pub(crate) fn passenger_factor(curve: &[(f32, f32)], hour: f32) -> f32 {
    let profile = opts_core::formats::map::DensityProfile {
        days: 0,
        points: curve.to_vec(),
    };
    profile.at(hour).max(0.0)
}

/// Draws one of the loaded characters, weighted by its share of the `humans.txt` lines.
fn draw_character(pool: &PedestrianPool, loaded: &[usize], rng: &mut u32) -> usize {
    let weights: Vec<usize> = loaded
        .iter()
        .map(|&i| {
            pool.entries
                .iter()
                .filter(|e| e.eq_ignore_ascii_case(&pool.slots[i].0))
                .count()
                .max(1)
        })
        .collect();
    let mut pick =
        (opts_sim::pedestrian::net::next_f32(rng) * weights.iter().sum::<usize>() as f32) as usize;
    let mut chosen = loaded[0];
    for (k, w) in weights.iter().enumerate() {
        if pick < *w {
            chosen = loaded[k];
            break;
        }
        pick -= w;
    }
    chosen
}

/// The walking speed (m/s): children and the very old walk more slowly, and everybody a little
/// differently.
fn walking_speed(age: Option<u32>, rng: &mut u32) -> f32 {
    let base = match age {
        Some(a) if a < 14 => 1.0,
        Some(a) if a > 70 => 0.9,
        _ => 1.25,
    };
    base + 0.3 * opts_sim::pedestrian::net::next_f32(rng)
}

/// The network, settings, pool and session facts the pedestrian spawner uses.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PedestrianWorld<'w> {
    net: Res<'w, PedestrianNetwork>,
    settings: Res<'w, PedestrianSettings>,
    pool: ResMut<'w, PedestrianPool>,
    bindposes: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    job: Option<Res<'w, super::super::WorldLoadJob>>,
    env: Option<Res<'w, opts_bevy::SessionEnvironment>>,
}

/// Spawns pedestrians on lanes around the camera up to the density-derived target.
pub fn spawn_pedestrians(
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut commands: Commands,
    world: PedestrianWorld,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    living: Query<(), With<Pedestrian>>,
) {
    let PedestrianWorld {
        net,
        settings,
        mut pool,
        mut bindposes,
        job,
        env,
    } = world;
    *elapsed += time.delta_secs();
    if *elapsed < SPAWN_INTERVAL_S || settings.baseline == 0 {
        return;
    }
    *elapsed = 0.0;
    let Ok(cam) = camera.single() else { return };
    // The map's `[trafficdensity_passenger]` curve scales the walkers by the time of day.
    let factor = match (&job, &env) {
        (Some(job), Some(env)) => {
            passenger_factor(&job.manifest.traffic_density_passenger, env.hour())
        }
        _ => 1.0,
    };
    let room = (net.0.total_length() / MIN_PATH_SPACING_M) as usize;
    let target = ((settings.baseline as f32 * factor) as usize).min(room);
    if living.iter().count() >= target {
        return;
    }
    // Draw a character, weighted by its share of the `humans.txt` lines.
    let loaded: Vec<usize> = pool
        .slots
        .iter()
        .enumerate()
        .filter(|(_, (_, v, _))| v.is_some())
        .map(|(i, _)| i)
        .collect();
    if loaded.is_empty() {
        return;
    }
    let centre = (cam.translation().x, -cam.translation().z);
    let mut rng = pool.rng;
    let Some((lane, s)) = net
        .0
        .random_spot(centre, SPAWN_MIN_M, SPAWN_MAX_M, &mut rng)
    else {
        pool.rng = rng;
        return;
    };
    let chosen = draw_character(&pool, &loaded, &mut rng);
    let age = pool.slots[chosen]
        .1
        .as_ref()
        .and_then(|v| v.profile.age_years);
    let speed = walking_speed(age, &mut rng);
    let walker = Walker::new(&net.0, lane, s, speed, &mut rng);
    pool.rng = rng;
    let Some(walker) = walker else { return };
    let Some(v) = pool.slots[chosen].1.as_ref() else {
        return;
    };
    spawn_walker(&mut commands, &mut bindposes, &net.0, v, walker);
}
