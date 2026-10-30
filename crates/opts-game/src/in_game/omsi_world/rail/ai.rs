//! AI trains and bikes on the rail network: the unit and train specs read from the vehicle files, the variants, spawning and moving them.

use super::obstacles::{nearest_gap, Obstacle};
use super::spec::{load_spec, load_zug_spec, TrainSpec};
use super::*;

/// Rail vehicles found in the map's AI lists.
#[derive(Resource, Default)]
pub struct RailAiPool {
    started: bool,
    specs: Vec<TrainSpec>,
    elapsed: f32,
    rng: u32,
    /// The vehicle files of every list group and depot group, by lower-case group name (the
    /// `vehicle_group` of a timetable tour).
    pub(super) group_vehicles: std::collections::HashMap<String, Vec<String>>,
    /// The vehicle files (lower case) that are rail vehicles.
    pub(super) rail_files: std::collections::HashSet<String>,
}

/// Riders found in the map's AI lists (light `[type] 2` vehicles: bicycles, e-scooters).
#[derive(Resource, Default)]
pub struct BikeAiPool(RailAiPool);

/// A running AI train.
#[derive(Component)]
pub struct AiTrain {
    pub(super) bike: bool,
    pub(super) train: Train,
    pub(super) speed: f32,
    pub(super) composition: VehicleComposition,
    /// Half bogie span of the head unit.
    pub(super) wheel_angle: f32,
    /// The timetable run it drives, if it is a scheduled train.
    pub(super) plan: Option<super::scheduled::RailPlan>,
}

/// Reads the AI lists once and keeps every rail vehicle (`[type] 2`) with its coupling chain.
pub fn load_rail_variants(
    job: Option<ResMut<WorldLoadJob>>,
    mut pool: ResMut<RailAiPool>,
    mut bikes: ResMut<BikeAiPool>,
) {
    let Some(job) = job else { return };
    if pool.started {
        return;
    }
    pool.started = true;
    pool.rng = 0x7A41_C0DE;
    bikes.0.started = true;
    bikes.0.rng = 0xB1CE_0001;
    // The lists in force on the map date (Chrono group updates merged in).
    let Some(bytes) = job.data_bytes("ailists.cfg") else {
        return;
    };
    let lists = opts_core::formats::map::AiLists::parse(&bytes);
    for g in &lists.groups {
        pool.group_vehicles.insert(
            g.name.to_ascii_lowercase(),
            g.entries.iter().map(|e| e.vehicle.clone()).collect(),
        );
    }
    for d in &lists.depot_groups {
        pool.group_vehicles
            .insert(d.name.to_ascii_lowercase(), d.vehicles.clone());
    }
    let mut seen = std::collections::HashSet::new();
    // A map names its own fleet: the depot vehicles (EVAG's NF6D on the tram map) or groups called
    // after trams. The rest of the lists (U-Bahn cars, regional trains) are traffic of other
    // networks; they are used only when the map names no fleet of its own.
    let tram_groups = |name: &str| {
        let n = name.to_ascii_lowercase();
        ["tram", "strassenbahn", "variobahn", "stadtbahn"]
            .iter()
            .any(|k| n.contains(k))
    };
    let mut candidates: Vec<(bool, &str)> = lists
        .depot_vehicles
        .iter()
        .map(|v| (true, v.as_str()))
        .collect();
    for g in &lists.groups {
        let own = tram_groups(&g.name);
        candidates.extend(g.entries.iter().map(|e| (own, e.vehicle.as_str())));
    }
    let mut trains: Vec<(bool, TrainSpec)> = Vec::new();
    for (own, vehicle) in candidates {
        let f = vehicle.to_ascii_lowercase();
        let zug = f.ends_with(".zug");
        if !(zug || f.ends_with(".bus") || f.ends_with(".ovh")) || !seen.insert(f.clone()) {
            continue;
        }
        let loaded = if zug {
            load_zug_spec(&job, vehicle)
        } else {
            load_spec(&job, vehicle)
        };
        if let Some((spec, light)) = loaded {
            if !light {
                pool.rail_files.insert(vehicle.to_ascii_lowercase());
            }
            if light {
                bikes.0.specs.push(spec);
            } else {
                trains.push((own, spec));
            }
        }
    }
    // Depot buses are road vehicles: only a rail fleet of the map's own counts as "named".
    let any_own = trains.iter().any(|(own, _)| *own);
    pool.specs = trains
        .into_iter()
        .filter(|(own, _)| *own || !any_own)
        .map(|(_, spec)| spec)
        .collect();
    tracing::info!(
        trains = pool.specs.len(),
        riders = bikes.0.specs.len(),
        "rail vehicles and cycles in the AI lists"
    );
}

fn next_rand(state: &mut u32) -> f32 {
    opts_sim::pedestrian::net::next_f32(state)
}

/// The settings, networks and spec pools of AI trains and cyclists.
#[derive(bevy::ecs::system::SystemParam)]
pub struct AiFleet<'w> {
    settings: Res<'w, RailSettings>,
    bike_settings: Res<'w, BikeSettings>,
    net: Res<'w, RailNetwork>,
    bike_net: Res<'w, BikeNetwork>,
    pool: ResMut<'w, RailAiPool>,
    bike_pool: ResMut<'w, BikeAiPool>,
}

/// The asset stores the AI vehicles' meshes are added to.
#[derive(bevy::ecs::system::SystemParam)]
pub struct AiAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    images: ResMut<'w, Assets<Image>>,
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

/// The canvases of a unit's script textures.
pub(super) type ScriptCanvases =
    std::sync::Arc<std::sync::RwLock<Vec<opts_core::formats::script_texture::CpuScriptTexture>>>;

pub(super) type SpawnAssets<'a> = (
    &'a mut Assets<Mesh>,
    &'a mut Assets<StandardMaterial>,
    &'a mut Assets<Image>,
);

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

/// The unit entities of a train spec with their meshes, hidden until placed; `None` (and
/// nothing left behind) while a mesh is still decoding in the background.
pub(super) fn build_train(
    spec: &TrainSpec,
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    assets: &mut SpawnAssets,
    scripted: bool,
) -> Option<(Entity, VehicleComposition, Vec<ScriptCanvases>)> {
    let (meshes, materials, images) = (&mut *assets.0, &mut *assets.1, &mut *assets.2);
    let mut units = Vec::new();
    let mut canvas_sets: Vec<ScriptCanvases> = Vec::new();
    for u in &spec.units {
        let root = commands
            .spawn((
                Transform::default(),
                Visibility::Hidden,
                Name::new(u.name.clone()),
                InGameEntity,
                opts_bevy::AiVariables::default(),
            ))
            .id();
        // A scripted unit draws its display on its own canvases; the others show no text.
        if scripted {
            job.vehicle_text_defs = u.defs.text.clone();
            job.vehicle_script_defs = u.defs.script.clone();
            let canvases = u
                .defs
                .script
                .iter()
                .map(|st| {
                    opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height)
                })
                .collect();
            job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
            canvas_sets.push(job.vehicle_script_textures.clone());
        } else {
            job.vehicle_text_defs.clear();
        }
        let (count, ..) = load_segment_meshes(
            job,
            commands,
            meshes,
            materials,
            images,
            root,
            &u.lod,
            &u.model_dir,
            &u.tex_dirs,
        );
        units.push((root, u.behind, u.reversed, u.span, count));
    }
    if units.iter().any(|u| u.4 == 0) {
        // Meshes still decoding in the background: discard and try again on the next tick.
        for (e, ..) in &units {
            commands.entity(*e).despawn();
        }
        return None;
    }
    let head = units[0].0;
    let part = |i: usize, u: &(Entity, f32, bool, f32, usize)| Part {
        entity: u.0,
        name: spec.units[i].name.clone(),
        behind: u.1,
        half_span: u.3,
        reversed: u.2,
        meshes: u.4,
        vertices: 0,
        body: spec.units[i].body,
    };
    let composition = VehicleComposition {
        head: part(0, &units[0]),
        followers: units
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, u)| part(i, u))
            .collect(),
    };
    Some((head, composition, canvas_sets))
}

mod step;
pub use step::move_ai_trains;
