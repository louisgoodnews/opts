//! Vehicle spawning, camera setup and final problem tally stages of the loading pipeline. The
//! camera choice of the controls stage is in [`camera`].

use super::*;

mod camera;

use camera::{requested_camera, setup_camera};

pub(super) fn stage_vehicle(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    if !state.vehicle_spawned {
        let LoadEnv {
            commands,
            assets,
            job,
            stats,
            report,
            setup,
            registry,
            ..
        } = env;
        if let Some(job) = job.as_deref_mut().filter(|j| !j.no_vehicle) {
            // A train composed in the wizard: the cars behind the chosen vehicle.
            let formation = crate::new_game_screen::apply_formation_at_start(setup, job);
            let params = SpawnParams {
                catalogue_id: job
                    .vehicle_file()
                    .and_then(|p| p.file_stem())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                name: setup.selected_vehicle.clone(),
                bus_file: job
                    .vehicle_file()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                repaint: job.config_livery().to_owned(),
                setvars: job.config_setvars().to_vec(),
                entry: job
                    .start_entry()
                    .map_or(EntrySel::Default, |(_, n)| EntrySel::Name(n.to_owned())),
                formation,
            };
            let id = registry.request_spawn(params);
            match load_player_vehicle(
                (job, stats, commands),
                (
                    &mut assets.meshes,
                    &mut assets.materials,
                    &mut assets.images,
                    &mut assets.bindposes,
                ),
                (id, true),
            ) {
                Ok(spawned) => {
                    let point = job.spawn_point().map(|(p, heading)| {
                        let abs = job.local_to_absolute(p.truncate());
                        SpawnPoint {
                            x: abs.x,
                            y: abs.y,
                            z: f64::from(p.z),
                            heading_deg: f64::from(heading),
                        }
                    });
                    let entry = job.start_entry().map(|(i, n)| (i, n.to_owned()));
                    if let Some(r) = registry.get_mut(id) {
                        r.name.clone_from(&spawned.name);
                    }
                    registry.mark_alive(id, spawned.root, entry, point, Some(spawned.camera));
                    registry.set_active(Some(id));
                }
                Err(e) => {
                    tracing::error!("cannot load player vehicle: {e}");
                    registry.mark_failed(id, e.clone());
                    report.add(
                        LoadProblem::new(false, "load-problem-vehicle", &setup.selected_vehicle, e)
                            .because("load-cause-vehicle-unreadable"),
                    );
                }
            }
        }
        state.vehicle_spawned = true;
    }
    state.progress = P_VEHICLE;
    state.stage = LoadStage::CalibrateControls;
}

pub(super) fn stage_controls(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    if !state.camera_spawned {
        let named = requested_camera(env.args.as_deref());
        if let Some(job) = env.job.as_deref() {
            setup_camera(
                &mut env.commands,
                job,
                &mut env.camera_state,
                named.as_deref(),
            );
        }
        spawn_debug_overlay(&mut env.commands);
        state.camera_spawned = true;
    }
    let (map, c) = (env.stats.map_name.clone(), counts(env, state));
    let failures = env
        .job
        .as_deref()
        .map(|j| j.failed_files(8).join("\n"))
        .unwrap_or_default();
    problems_from_counts(&mut env.report, &map, c, &failures);
    env.report.finish();
    env.stats.load_problems = env.report.count();
    state.progress = 1.0;
    state.stage = LoadStage::Ready;
    state.is_complete = true;
}

fn counts(env: &LoadEnv, state: &SimulationLoadState) -> WorldCounts {
    WorldCounts {
        tiles_loaded: env.stats.tiles_loaded,
        tiles_missing: env.stats.tiles_missing,
        tiles_total: state.tiles_total
            + env
                .job
                .as_deref()
                .map_or(0, WorldLoadJob::tiles_listed_missing),
        objects_failed: env.stats.objects_failed + env.stats.splines_failed,
        asset_failures: env.stats.asset_failures,
    }
}
