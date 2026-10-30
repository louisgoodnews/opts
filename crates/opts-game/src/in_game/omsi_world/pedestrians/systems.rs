//! Spawning, walking and configuring pedestrians around the camera.

use super::*;

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

/// Spawns the character as a standing human on the walker's path and records it in the scene trace.
fn spawn_walker(
    commands: &mut Commands,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    net: &PedestrianNet,
    v: &Variant,
    walker: Walker,
) {
    let leg_m = rest_pivots(&v.profile)[0].y.max(0.3);
    let root = spawn_human(
        commands,
        bindposes,
        &v.profile,
        &v.parts,
        &v.name,
        HumanPose::Standing,
    );
    if opts_bevy::scene_trace::enabled() {
        let frame = super::super::trace::human_frame(&v.name);
        let tf = Transform::from_translation(
            walker
                .pose(net)
                .map(|(p, _)| omsi_to_bevy_pos(p))
                .unwrap_or_default(),
        );
        super::super::trace::spawned(
            opts_bevy::scene_trace::kind::HUMAN,
            &v.name,
            &v.name,
            root,
            &tf,
            &frame,
            "ok",
            serde_json::json!({}),
        );
    }
    commands.entity(root).insert((
        Pedestrian { walker, leg_m },
        Visibility::default(),
        InGameEntity,
    ));
}

/// Spawns pedestrians on lanes around the camera up to the density-derived target.
#[allow(clippy::too_many_arguments)]
pub fn spawn_pedestrians(
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut commands: Commands,
    net: Res<PedestrianNetwork>,
    settings: Res<PedestrianSettings>,
    mut pool: ResMut<PedestrianPool>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    living: Query<(), With<Pedestrian>>,
    job: Option<Res<super::WorldLoadJob>>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
) {
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

/// Walks the pedestrians, poses them and removes finished or distant ones.
pub fn move_pedestrians(
    time: Res<Time>,
    mut commands: Commands,
    net: Res<PedestrianNetwork>,
    mut pool: ResMut<PedestrianPool>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut peds: Query<(Entity, &mut Pedestrian, &Human)>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs().min(0.1);
    let cam = camera.single().ok().map(|c| c.translation());
    let mut rng = pool.rng ^ 0x5bd1_e995;
    for (entity, mut ped, human) in &mut peds {
        let outcome = ped.walker.advance(dt, &net.0, &mut rng);
        let Some((pos, heading)) = ped.walker.pose(&net.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        let bevy_pos = omsi_to_bevy_pos(pos);
        let far = cam.is_some_and(|c| c.distance(bevy_pos) > DESPAWN_M);
        if outcome == Advance::Finished || far {
            commands.entity(entity).despawn();
            continue;
        }
        if let Ok(mut t) = transforms.get_mut(entity) {
            t.translation = bevy_pos;
            t.rotation = omsi_euler_to_bevy_quat(-heading.to_degrees(), 0.0, 0.0);
        }
        // Gait.
        ped.walker.phase = advance_phase(human, ped.walker.phase, ped.walker.speed, dt);
        apply_gait(human, ped.leg_m, ped.walker.phase, &mut transforms);
    }
    pool.rng = rng;
}

/// Reads `--pedestrians` and resets the per-session state when a session starts.
pub fn configure_pedestrians(
    args: Option<Res<crate::AppArgs>>,
    user: Option<Res<crate::settings::GameSettings>>,
    mut settings: ResMut<PedestrianSettings>,
    mut net: ResMut<PedestrianNetwork>,
    mut pool: ResMut<PedestrianPool>,
    mut ground: ResMut<super::super::GroundIndex>,
) {
    *ground = super::super::GroundIndex::default();
    // The command line wins for this run; then the setting *Pedestrians*; then the default.
    settings.baseline = args
        .and_then(|a| a.0.pedestrians)
        .or_else(|| user.and_then(|u| u.raw("gameplay.pedestrians")?.parse().ok()))
        .unwrap_or(DEFAULT_PEDESTRIAN_BASELINE);
    net.0 = PedestrianNet::default();
    *pool = PedestrianPool::default();
}
