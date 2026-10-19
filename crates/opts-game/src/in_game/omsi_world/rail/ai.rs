//! AI trains and bikes on the rail network: the unit and train specs read from the vehicle files, the variants, spawning and moving them.

use super::*;

/// One unit of a consist.
struct UnitSpec {
    name: String,
    lod: ModelLod,
    model_dir: std::path::PathBuf,
    tex_dirs: Vec<std::path::PathBuf>,
    /// Distance of the unit's origin behind the head unit's origin (m).
    behind: f32,
    /// Hangs backwards (coupled with the `true` flag).
    reversed: bool,
    /// Half bogie span (`[boogies]`; 0 when unknown).
    span: f32,
}

/// A trainset assembled from a `.bus`/`.ovh` chain.
struct TrainSpec {
    name: String,
    units: Vec<UnitSpec>,
}

/// Rail vehicles found in the map's AI lists.
#[derive(Resource, Default)]
pub struct RailAiPool {
    started: bool,
    specs: Vec<TrainSpec>,
    elapsed: f32,
    rng: u32,
}

/// Riders found in the map's AI lists (light `[type] 2` vehicles: bicycles, e-scooters).
#[derive(Resource, Default)]
pub struct BikeAiPool(RailAiPool);

/// A running AI train.
#[derive(Component)]
pub struct AiTrain {
    bike: bool,
    train: Train,
    speed: f32,
    composition: VehicleComposition,
    /// Half bogie span of the head unit.
    wheel_angle: f32,
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
    let mut seen = std::collections::HashSet::new();
    for g in &lists.groups {
        for e in &g.entries {
            let f = e.vehicle.to_ascii_lowercase();
            if !(f.ends_with(".bus") || f.ends_with(".ovh")) || !seen.insert(f) {
                continue;
            }
            if let Some((spec, light)) = load_spec(&job, &e.vehicle) {
                if light {
                    bikes.0.specs.push(spec);
                } else {
                    pool.specs.push(spec);
                }
            }
        }
    }
    tracing::info!(
        trains = pool.specs.len(),
        riders = bikes.0.specs.len(),
        "rail vehicles and cycles in the AI lists"
    );
}

fn parse_unit(
    job: &WorldLoadJob,
    path: &Path,
) -> Option<(
    BusManifest,
    ModelLod,
    std::path::PathBuf,
    Vec<std::path::PathBuf>,
)> {
    let manifest = BusManifest::parse_bytes(&std::fs::read(path).ok()?).ok()?;
    let dir = path.parent()?.to_path_buf();
    let cfg = job.install.find_in(&dir, manifest.model_cfg.as_deref()?)?;
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).ok()?).ok()?;
    let lod = model
        .lods
        .iter()
        .find(|l| l.threshold == 0.0)
        .or_else(|| model.lods.first())?
        .clone();
    let model_dir = cfg.parent()?.to_path_buf();
    let tex_dirs = vec![
        job.install
            .find_in(&dir, "texture")
            .unwrap_or_else(|| dir.join("texture")),
        model_dir.join("texture"),
    ];
    Some((manifest, lod, model_dir, tex_dirs))
}

/// The coupling chain of a `[type] 2` vehicle and whether it is a light one (bicycle, e-scooter:
/// under a tonne; OMSI files them under the rail type too); `None` for road vehicles and
/// unreadable files.
fn load_spec(job: &WorldLoadJob, rel: &str) -> Option<(TrainSpec, bool)> {
    let path = job.install.find_file(rel)?;
    let (mut manifest, lod, model_dir, tex_dirs) = parse_unit(job, &path)?;
    if !is_rail(&manifest) {
        return None;
    }
    let light = manifest.mass_tonnes.is_some_and(|m| m < 1.0);
    let name = path.file_stem()?.to_string_lossy().into_owned();
    let mut units = vec![UnitSpec {
        name: name.clone(),
        lod,
        model_dir,
        tex_dirs,
        behind: 0.0,
        reversed: false,
        span: manifest.bogie_half_span.unwrap_or(0.0),
    }];
    let mut dir = path.parent()?.to_path_buf();
    let mut behind = 0.0_f32;
    for _ in 0..40 {
        let Some(next) = manifest.couple_back.clone() else {
            break;
        };
        let Some(next_path) = job.install.find_in(&dir, &next) else {
            break;
        };
        let Some((m, lod, model_dir, tex_dirs)) = parse_unit(job, &next_path) else {
            break;
        };
        let reversed = manifest.couple_back_reversed;
        let Some((b, f)) = super::super::vehicle::coupling_joint(&manifest, false, &m, reversed)
        else {
            break;
        };
        // The next unit's origin lies at `b − f` in this unit's frame (y forward).
        behind += f.y - b.y;
        units.push(UnitSpec {
            name: next_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            lod,
            model_dir,
            tex_dirs,
            behind,
            reversed,
            span: m.bogie_half_span.unwrap_or(0.0),
        });
        if reversed {
            break; // the ring of a double-ended consist closes here
        }
        dir = next_path.parent()?.to_path_buf();
        manifest = m;
    }
    Some((TrainSpec { name, units }, light))
}

fn next_rand(state: &mut u32) -> f32 {
    opts_sim::pedestrian::net::next_f32(state)
}

/// Spawns AI trains on tracks and cyclists on cycle paths around the camera.
#[allow(clippy::too_many_arguments)]
pub fn spawn_ai_trains(
    time: Res<Time>,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    settings: Res<RailSettings>,
    bike_settings: Res<BikeSettings>,
    net: Res<RailNetwork>,
    bike_net: Res<BikeNetwork>,
    mut pool: ResMut<RailAiPool>,
    mut bike_pool: ResMut<BikeAiPool>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    living: Query<&AiTrain>,
) {
    let (Some(mut job), Ok(cam)) = (job, camera.single()) else {
        return;
    };
    let centre = (cam.translation().x, -cam.translation().z);
    let trains = living.iter().filter(|t| !t.bike).count();
    let riders = living.iter().count() - trains;
    let dt = time.delta_secs();
    let mut assets = (&mut *meshes, &mut *materials, &mut *images);
    try_spawn(
        false,
        dt,
        settings.max_trains.saturating_sub(trains),
        centre,
        &net.0,
        &mut pool,
        &mut job,
        &mut commands,
        &mut assets,
    );
    try_spawn(
        true,
        dt,
        bike_settings.max_riders.saturating_sub(riders),
        centre,
        &bike_net.0,
        &mut bike_pool.0,
        &mut job,
        &mut commands,
        &mut assets,
    );
}

type SpawnAssets<'a> = (
    &'a mut Assets<Mesh>,
    &'a mut Assets<StandardMaterial>,
    &'a mut Assets<Image>,
);

/// One spawn attempt per second and kind, when below the cap.
#[allow(clippy::too_many_arguments)]
fn try_spawn(
    bike: bool,
    dt: f32,
    room: usize,
    centre: (f32, f32),
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
    let (meshes, materials, images) = (&mut *assets.0, &mut *assets.1, &mut *assets.2);
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
    let mut units = Vec::new();
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
        job.vehicle_text_defs.clear();
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
        return;
    }
    tracing::info!(vehicle = %spec.name, units = spec.units.len(), lane, bike, "AI rail/cycle vehicle spawned");
    let head = units[0].0;
    let part = |i: usize, u: &(Entity, f32, bool, f32, usize)| Part {
        entity: u.0,
        name: spec.units[i].name.clone(),
        behind: u.1,
        half_span: u.3,
        reversed: u.2,
        meshes: u.4,
        vertices: 0,
        body: None,
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
    commands.entity(head).insert(AiTrain {
        bike,
        train: Train::new(lane, s, dir),
        speed: 0.0,
        composition,
        wheel_angle: 0.0,
    });
}

/// Runs the AI trains along their track and poses every unit by its distance behind the head.
pub fn move_ai_trains(
    time: Res<Time>,
    mut commands: Commands,
    rail_net: Res<RailNetwork>,
    bike_net: Res<BikeNetwork>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut trains: Query<(Entity, &mut AiTrain)>,
    mut units: Query<(&mut Transform, &mut Visibility, &mut opts_bevy::AiVariables)>,
) {
    let dt = time.delta_secs().min(0.1);
    let cam = camera.single().ok().map(|c| c.translation());
    for (head, mut t) in &mut trains {
        let net = if t.bike { &bike_net.0 } else { &rail_net.0 };
        let (accel, cruise) = if t.bike {
            (1.0, BIKE_CRUISE_MS)
        } else {
            (0.8, AI_CRUISE_MS)
        };
        t.speed = (t.speed + accel * dt).min(cruise);
        let step = t.speed * dt;
        let end = t.train.advance(step, net) == Moved::EndOfTrack;
        let far = cam.is_some_and(|c| {
            t.train
                .pose(net)
                .is_some_and(|(p, _)| c.distance(omsi_to_bevy_pos(p)) > AI_DESPAWN_M)
        });
        if end || far || t.train.pose(net).is_none() {
            for p in t.composition.parts() {
                commands.entity(p.entity).despawn();
            }
            let _ = head;
            continue;
        }
        t.wheel_angle += step / 0.29;
        let angle = t.wheel_angle;
        let head_span = t.composition.head.half_span;
        let list: Vec<Part> = t.composition.parts().cloned().collect();
        for part in list {
            let (e, reversed) = (part.entity, part.reversed);
            let Some(pose) = unit_pose(
                &t.train,
                net,
                part.behind + head_span,
                part.half_span,
                reversed,
                0.0,
            ) else {
                continue;
            };
            if let Ok((mut tf, mut vis, mut vars)) = units.get_mut(e) {
                *tf = pose;
                *vis = Visibility::Visible;
                for axle in 0..6 {
                    for side in ["l", "r"] {
                        vars.set(&format!("wheel_rotation_{axle}_{side}"), angle);
                    }
                }
            }
        }
    }
}
