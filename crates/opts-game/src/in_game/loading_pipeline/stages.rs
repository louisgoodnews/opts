//! Vehicle spawning, camera setup and final problem tally stages of the loading pipeline.

use super::*;

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
                job,
                stats,
                commands,
                &mut assets.meshes,
                &mut assets.materials,
                &mut assets.images,
                &mut assets.bindposes,
                id,
                true,
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

/// The camera mode a `--camera` argument asks for, if any.
fn requested_camera(args: Option<&AppArgs>) -> Option<String> {
    args.and_then(|a| a.0.camera.as_deref())
        .map(str::to_ascii_lowercase)
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

/// Chooses the starting camera and mode from the loaded vehicle (or an inspection camera).
fn setup_camera(
    commands: &mut Commands,
    job: &WorldLoadJob,
    camera_state: &mut CameraState,
    named: Option<&str>,
) {
    if let Some(cam) = job.camera {
        spawn_camera_rig(commands, &cam);
        camera_state.mode = match named {
            Some("cockpit") => {
                camera_state.sub_camera_index = job.camera_std_index;
                CameraMode::Cockpit
            }
            Some("pax") => CameraMode::Passenger,
            Some("ego") => CameraMode::Ego,
            Some("free") => free_roam_start(commands, camera_state, &cam),
            _ => CameraMode::External,
        };
    } else if let Some(tf) = job.inspection_camera() {
        // No vehicle: still give the user a camera to inspect the loaded world.
        tracing::warn!("no vehicle loaded; starting in free-roam over the spawn point");
        spawn_free_camera(commands, tf);
        camera_state.mode = CameraMode::FreeRoam;
        camera_state.free_roam_spawn = tf.translation;
        let (yaw, pitch, _) = tf.rotation.to_euler(EulerRot::YXZ);
        camera_state.free_roam_yaw = yaw;
        camera_state.free_roam_pitch = pitch;
    }
    camera_state.previous_mode = camera_state.mode;
    tracing::info!(mode = ?camera_state.mode, "initial camera mode");
}

/// Free-roam start. Developer aid: `OPTS_DEV_CAM_HEIGHT=<m>` starts high above the spawn,
/// looking down (overview screenshots).
fn free_roam_start(
    commands: &mut Commands,
    camera_state: &mut CameraState,
    cam: &crate::in_game::omsi_world::CameraSetup,
) -> CameraMode {
    let mut start = cam.start;
    if let Some(h) = std::env::var("OPTS_DEV_CAM_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        let focus = cam.start.translation + *cam.start.forward() * h * 0.5;
        start = Transform::from_translation(cam.start.translation + Vec3::Y * h).looking_at(
            Vec3::new(focus.x, cam.start.translation.y, focus.z),
            Vec3::Y,
        );
        commands.queue(move |w: &mut World| {
            let mut q = w.query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
            for mut t in q.iter_mut(w) {
                *t = start;
            }
        });
    }
    camera_state.free_roam_spawn = start.translation;
    let (yaw, pitch, _) = start.rotation.to_euler(EulerRot::YXZ);
    camera_state.free_roam_yaw = yaw;
    camera_state.free_roam_pitch = pitch;
    CameraMode::FreeRoam
}
