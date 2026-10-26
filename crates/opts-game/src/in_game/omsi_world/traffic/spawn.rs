//! Spawning and despawning traffic.

use super::variants::{
    baseline_targets, group_lane_metres, pick_slot, time_factor, unit_lod, unit_size,
};
use super::*;

/// Spawns vehicles on lanes around the camera up to the density-derived target.
#[allow(clippy::too_many_arguments)]
pub fn spawn_traffic(
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut since_log: Local<f32>,
    mut rejected: Local<[u32; 5]>,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    mut state: ResMut<TrafficState>,
    mut pool: ResMut<TrafficPool>,
    settings: Res<TrafficSettings>,
    mut sched: ResMut<ScheduledTraffic>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    chrono: Option<Res<crate::in_game::omsi_world::ChronoState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) {
    *elapsed += time.delta_secs();
    if *elapsed < SPAWN_INTERVAL_S || settings.baseline == 0 {
        return;
    }
    *elapsed = 0.0;
    let (Some(mut job), Ok(cam)) = (job, camera.single()) else {
        return;
    };
    let centre = DVec2::new(
        f64::from(cam.translation().x),
        -f64::from(cam.translation().z),
    );
    // Density model: per group, the lane metres within the despawn disc weighted by the group's
    // density on each lane, times its time factor for the session's hour and day, per `L0`.
    let (hour, day_bit) = env.map_or((12.0, day::WEEKDAY), |e| (e.hour(), e.day_bit()));
    let factors: Vec<f64> = pool
        .groups
        .iter()
        .map(|g| time_factor(g, &pool.road_curve, day_bit, hour))
        .collect();
    let metres = group_lane_metres(
        state.world.graph(),
        centre,
        f64::from(DESPAWN_M),
        &pool.groups,
        &pool.defaults,
    );
    // Depot groups drive timetable runs only, never as unscheduled traffic.
    let metres: Vec<f64> = metres
        .iter()
        .zip(&pool.groups)
        .map(|(m, g)| if g.scheduled { 0.0 } else { *m })
        .collect();
    // The baseline setting spread over the groups, scaled by the map's factors for this hour and
    // day, limited to what the roads hold.
    let (targets, capped) = baseline_targets(&metres, &factors, settings.baseline);
    let wanted: Vec<f64> = metres
        .iter()
        .zip(&factors)
        .map(|(m, f)| m * f / METRES_OF_LANE_PER_VEHICLE)
        .collect();
    if capped != pool.capped {
        pool.capped = capped;
        if capped {
            tracing::info!(
                baseline = settings.baseline,
                room = ?targets.iter().sum::<f64>(),
                "the roads are full: every group scaled down in proportion"
            );
        } else {
            tracing::info!(
                baseline = settings.baseline,
                "the roads hold the baseline again"
            );
        }
    }
    let target = targets.iter().sum::<f64>().round() as usize;
    // Vehicles per group.
    let TrafficPool {
        agent_groups,
        groups,
        ..
    } = &mut *pool;
    agent_groups.retain(|id, _| state.world.agent(*id).is_some());
    let mut alive = vec![0usize; groups.len()];
    for &g in agent_groups.values() {
        if let Some(a) = alive.get_mut(g) {
            *a += 1;
        }
    }
    let deficits: Vec<f64> = targets
        .iter()
        .zip(&alive)
        .map(|(t, &a)| t - a as f64)
        .collect();
    // A line every 15 s says why no vehicle appears (no road, no density, vehicles not loaded).
    *since_log += SPAWN_INTERVAL_S;
    if *since_log >= 15.0 {
        *since_log = 0.0;
        let count = |f: fn(&SlotState) -> bool| pool.slots.iter().filter(|s| f(&s.state)).count();
        tracing::debug!(
            target,
            agents = state.world.agents().len(),
            road_km = state.road_length_m / 1000.0,
            hour,
            day_bit,
            time_factors = ?factors,
            wanted = ?wanted,
            alive = ?alive,
            capped,
            ready = count(|s| matches!(s, SlotState::Ready(_))),
            pending = count(|s| matches!(s, SlotState::Pending)),
            failed = count(|s| matches!(s, SlotState::Failed)),
            rejected = ?*rejected,
            "traffic state"
        );
    }
    // A scheduled vehicle asked for by `plan_scheduled` goes first and does not count against the
    // target of the unscheduled traffic.
    let request = sched.request.take();
    let unscheduled = state
        .world
        .agents()
        .len()
        .saturating_sub(sched.agent_count());
    if request.is_none() && unscheduled >= target {
        return;
    }
    let mut rng = pool.rng.take().unwrap_or_else(|| Rng::new(1));
    // Why an attempt found no place, counted for the diagnostic line: no vehicle ready (or no
    // group below its target), no lane in the ring around the camera, lane gone, -, in the
    // camera's view.
    let requested = request.as_ref().and_then(|r| {
        let at = state.world.graph().lane(r.lane)?.geometry.point_at(r.s);
        Some((r.lane, r.s, r.slot, at))
    });
    let result = (|| -> Result<_, usize> {
        if let Some(found) = requested {
            return Ok(found);
        }
        let fwd = cam.forward();
        let view = DVec2::new(f64::from(fwd.x), -f64::from(fwd.z)).normalize_or_zero();
        // The group and vehicle type come first: bus and truck lanes admit only their own, and
        // the lane is drawn by length × the group's density on it.
        let slot = pick_slot(&pool, &deficits, &mut rng).ok_or(0_usize)?;
        let class = match &pool.slots[slot].state {
            SlotState::Ready(v) => v.class(),
            _ => VehicleClass::Car,
        };
        let group = pool.groups[pool.slots[slot].group].density_group;
        let defaults = &pool.defaults;
        let (lane_id, s) = state
            .world
            .graph()
            .weighted_spot(centre, SPAWN_MIN_M, SPAWN_MAX_M, &mut rng, |l| {
                if l.kind == LaneKind::Road && l.length() > 6.0 && l.rules.allows(class) {
                    l.length() * l.rules.density.for_group(group, defaults)
                } else {
                    0.0
                }
            })
            .ok_or(1_usize)?;
        let lane = state.world.graph().lane(lane_id).ok_or(2_usize)?;
        let at = lane.geometry.point_at(s);
        // Not in plain view of the camera.
        let to = (at - centre).normalize_or_zero();
        if view.dot(to) > 0.5 && at.distance(centre) < 160.0 {
            return Err(4);
        }
        Ok((lane_id, s, slot, at))
    })();
    let (lane_id, s, slot, at) = match result {
        Ok(found) => found,
        Err(reason) => {
            rejected[reason] += 1;
            pool.rng = Some(rng);
            return;
        }
    };

    let pool_group = pool.slots[slot].group;
    let group_name = pool.groups[pool_group].name.clone();
    let density_group = u8::try_from(pool.groups[pool_group].density_group).unwrap_or(u8::MAX);
    pool.built += 1;
    let repaint_seed = crate::new_game_screen::repaints::instance_seed(&[slot as u64, pool.built]);
    // Build the models of every unit (parts are cached, so repeated types are cheap).
    let SlotState::Ready(variant) = &pool.slots[slot].state else {
        pool.rng = Some(rng);
        if let Some(r) = &request {
            sched.failed(r);
        }
        return;
    };
    let origin = Transform::from_translation(omsi_to_bevy_pos(at.extend(0.0).as_vec3()));
    let entities: Vec<Entity> = variant
        .units
        .iter()
        .map(|u| {
            commands
                .spawn((
                    origin,
                    Visibility::Hidden,
                    Name::new(u.name.clone()),
                    InGameEntity,
                ))
                .id()
        })
        .collect();
    let root = entities[0];
    super::super::trace::begin_frame();
    let mut bounds = Vec::with_capacity(entities.len());
    let mut failed = None;
    // A scheduled vehicle runs its scripts: the canvases of the head's script textures are the
    // ones its scripts draw on.
    let mut head_canvases = None;
    let head_bus_path = variant.units[0].bus_path.clone();
    // A depot bus wears the repaint its fleet line names (`ailists.cfg`: number, plate,
    // repaint); a vehicle without such a repaint, and unscheduled traffic, a seeded choice.
    let fleet_repaint = {
        let today = chrono
            .as_ref()
            .and_then(|c| c.date)
            .map(opts_core::formats::chrono::ChronoDate::yyyymmdd);
        // The vehicles in service on the map date (all of them without a date or when none is).
        let all = &pool.slots[slot].fleet;
        let in_service: Vec<_> = all
            .iter()
            .filter(|e| today.is_none_or(|d| e.in_service(d)))
            .collect();
        let fleet = if in_service.is_empty() {
            all.iter().collect()
        } else {
            in_service
        };
        (!fleet.is_empty())
            .then(|| fleet[usize::try_from(repaint_seed % fleet.len() as u64).unwrap_or(0)])
            .map(|e| e.repaint.clone())
    };
    let pick_repaint = |repaints: &crate::new_game_screen::repaints::VehicleRepaints| -> String {
        fleet_repaint
            .as_deref()
            .and_then(|w| repaints.named(w))
            .unwrap_or_else(|| repaints.pick(repaint_seed))
            .to_owned()
    };
    let head_setvars = {
        let repaints = &variant.units[0].repaints;
        let picked = pick_repaint(repaints);
        let picked = picked.as_str();
        let name = if picked.is_empty() {
            repaints.default_name.as_str()
        } else {
            picked
        };
        repaints.setvars.get(name).cloned().unwrap_or_default()
    };
    for (unit, &entity) in variant.units.iter().zip(&entities) {
        let Some(lod) = unit_lod(unit) else {
            failed = Some(true);
            break;
        };
        job.vehicle_text_defs = unit.model.text_textures.clone();
        if request.is_some() {
            job.vehicle_script_defs = unit.model.script_textures.clone();
            let canvases = unit
                .model
                .script_textures
                .iter()
                .map(|st| {
                    opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height)
                })
                .collect();
            job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
            if entity == root {
                head_canvases = Some(job.vehicle_script_textures.clone());
            }
        }
        // Each vehicle wears one of its `[CTC]` colour schemes (all units the same seed).
        job.cache.texture_replacements = unit
            .repaints
            .replacements(&pick_repaint(&unit.repaints), &unit.model.ctc_textures)
            .into_iter()
            .collect();
        let (count, _, min, max) = load_segment_meshes_for(
            &mut job,
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            entity,
            root,
            &lod,
            &unit.model_dir.clone(),
            &unit.tex_dirs.clone(),
        );
        job.cache.texture_replacements.clear();
        if count == 0 {
            failed = Some(false);
            break;
        }
        bounds.push((min, max));
    }
    let trace_frame = super::super::trace::end_frame();
    if let Some(unusable) = failed {
        // No usable meshes (unusable), or still decoding in the background: try again a few times.
        let name = variant.name.clone();
        for e in entities {
            commands.entity(e).despawn();
        }
        tracing::debug!(vehicle = %name, "AI vehicle meshes not ready");
        if let Some(r) = &request {
            sched.failed(r);
        }
        if unusable {
            pool.slots[slot].state = SlotState::Failed;
        } else {
            pool.slots[slot].fails += 1;
            if pool.slots[slot].fails >= MAX_LOAD_TRIES {
                pool.slots[slot].state = SlotState::Failed;
            }
        }
        pool.rng = Some(rng);
        return;
    }
    let SlotState::Ready(variant) = &mut pool.slots[slot].state else {
        return;
    };
    for (unit, (min, max)) in variant.units.iter_mut().zip(&bounds) {
        let boxed = unit.profile.body_length_m;
        unit.size
            .get_or_insert_with(|| unit_size(min.z, max.z, boxed));
    }
    let traits = variant.traits();
    let mut params = IdmParams::from_traits(&traits, &Calibration::default(), rng.next_f64());
    // A scheduled vehicle follows its timetable, not a traffic density: it belongs to no group,
    // so a lane the cars' group is closed on (a bus lane) is open to it.
    params.density_group = if request.is_some() {
        u8::MAX
    } else {
        density_group
    };
    // The vehicle may not use the lane it was placed on (its length was unknown before).
    if state
        .world
        .graph()
        .lane(lane_id)
        .is_some_and(|l| !l.rules.allows(params.class))
    {
        for e in entities {
            commands.entity(e).despawn();
        }
        pool.rng = Some(rng);
        if let Some(r) = &request {
            sched.failed(r);
        }
        return;
    }
    let speed = match &request {
        Some(r) => r.speed.min(state.world.lane_cap(&params, lane_id)),
        None => state.world.lane_cap(&params, lane_id) * 0.8,
    };
    let head = &variant.units[0];
    let sound = head.sound.clone();
    // A bus stops with its front door at the stop (the middle of the waiting area), so its front
    // bumper stands that far ahead of it.
    let door_ahead = head
        .front_door_y
        .map_or(0.0, |y| (head.size.unwrap_or((4.4, 2.2)).1 - y).max(0.0));
    // The sound sets of the following units (the second unit of an articulated bus).
    let unit_sounds: Vec<Option<std::sync::Arc<crate::vehicle_sound::AiSoundSet>>> = variant
        .units
        .iter()
        .skip(1)
        .map(|u| u.sound.clone())
        .collect();
    let size = head.size.unwrap_or((4.4, 2.2));
    let heavy = params.class != VehicleClass::Car;
    let wheel_radius = head
        .profile
        .wheel_radius_m
        .unwrap_or(if heavy { 0.5 } else { 0.31 });
    // Model frame: front bumper at `size.1`, front axle the overhang behind it, rear axle a
    // wheelbase further back.
    let rear_axle_y = size.1 - (params.front_overhang + params.wheelbase) as f32;
    let delta_height = head.profile.delta_height_m;
    let name = variant.name.clone();
    let fallbacks = head.profile.fallbacks.clone();
    let measured = head.response.is_some();
    let chain: Vec<ChainUnit> = variant
        .units
        .iter()
        .zip(&entities)
        .skip(1)
        .map(|(u, &e)| {
            ChainUnit::new(
                e,
                u.hitch,
                u.joint,
                u.rear_axle_y,
                u.max_alpha_deg,
                u.reversed,
            )
        })
        .collect();
    // Light effects (`[light_enh_2]`: headlights, tail and brake lights, indicators) of every
    // unit, driven by the head's AI variables.
    for (unit, &e) in variant.units.iter().zip(&entities) {
        opts_bevy::spawn_vehicle_glows(&mut commands, root, e, &unit.model);
    }
    let mut rng_pax = Rng::new(rng.next_u64());
    pool.rng = Some(rng);
    let mut scripted_head = false;
    match state.world.spawn(lane_id, s, speed, params) {
        Ok(id) => {
            pool.agent_groups.insert(id, pool_group);
            if let Some(r) = &request {
                state.world.set_route(id, r.route.iter().copied());
                for stop in &r.stops {
                    let mut stop = *stop;
                    let room = state
                        .world
                        .graph()
                        .lane(stop.lane)
                        .map_or(stop.s, |l| (l.length() - 0.5).max(0.0));
                    stop.s = (stop.s + f64::from(door_ahead)).min(room);
                    state.world.push_stop(id, stop);
                }
                sched.built(id, r);
                if let Some(canvases) = head_canvases.take() {
                    let seed = id.0.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ r.run.run as u64;
                    let scripted = super::ai_script::attach(
                        &mut job,
                        &mut commands,
                        root,
                        &head_bus_path,
                        canvases,
                        &r.run,
                        seed,
                        &head_setvars,
                    );
                    // The sounds that need the scripts' variables and events (the doors) play
                    // from a vehicle that runs them.
                    if let (true, Some(set)) = (scripted, sound.as_ref()) {
                        crate::player_sound::attach_defs(&mut commands, root, None, &set.scripted);
                    }
                    scripted_head = scripted;
                }
            }
            tracing::debug!(
                agent = id.0,
                vehicle = %name,
                group = %group_name,
                speed,
                mass_kg = traits.mass_kg,
                desired_kmh = params.desired_speed * 3.6,
                accel = params.max_accel,
                comfort = params.comfort_decel,
                max_decel = params.max_decel,
                class = ?params.class,
                units = entities.len(),
                measured,
                fallbacks = ?fallbacks,
                "AI vehicle spawned"
            );
            if opts_bevy::scene_trace::enabled() {
                super::super::trace::spawned(
                    opts_bevy::scene_trace::kind::TRAFFIC,
                    &name,
                    &name,
                    root,
                    &origin,
                    &trace_frame,
                    "ok",
                    serde_json::json!({
                        "lane": format!("{lane_id:?}"),
                        "speed": speed,
                        "class": format!("{:?}", params.class),
                        "units": entities.len(),
                    }),
                );
            }
            // Seats: the driver is nearly always there, further passengers by chance.
            let mut pax = 0u8;
            for (i, p) in [0.9, 0.35, 0.2, 0.1].into_iter().enumerate() {
                if rng_pax.next_f64() < p {
                    pax |= 1 << i;
                }
            }
            commands.entity(root).insert((
                TrafficVehicle {
                    id,
                    delta_height,
                    wheel_radius,
                    rear_axle_y,
                    wheel_angle: 0.0,
                    pax,
                    blink_phase: rng_pax.next_f64() as f32 * 0.7,
                    origin: at,
                    heading: 0.0,
                    height: delta_height,
                    honk_after: HONK_AFTER_S,
                },
                opts_bevy::AiVariables::default(),
            ));
            crate::vehicle_sound::attach_ai(&mut commands, root, sound.as_ref());
            // The following units of an articulated bus have sound configurations of their own
            // (`sound_*_T_ai.cfg`, at their own places) driven by the head's variables, and the
            // door sounds of their doors from the head's scripts.
            for (set, &e) in unit_sounds.iter().zip(entities.iter().skip(1)) {
                let Some(set) = set else {
                    continue;
                };
                crate::vehicle_sound::attach_ai_unit(&mut commands, e, Some(root), set);
                if scripted_head {
                    crate::player_sound::attach_defs(&mut commands, e, Some(root), &set.scripted);
                }
            }
            if !chain.is_empty() {
                commands.entity(root).insert(TrafficChain { units: chain });
            }
        }
        Err(_) => {
            for e in entities {
                commands.entity(e).despawn();
            }
            if let Some(r) = &request {
                sched.failed(r);
            }
        }
    }
}
