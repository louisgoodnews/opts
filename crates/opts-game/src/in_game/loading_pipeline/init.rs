//! The init stage of the loading pipeline: catalogue wait, session set-up and the first load requests.

use super::*;

pub(super) fn stage_init(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    state.waiting_for_catalogue = catalogue_pending(env);
    if state.waiting_for_catalogue {
        return;
    }
    let mut setup = env.setup.clone();
    // `--entry` and the protocol's `entry` name the start point of the map that is loaded; the
    // wizard's list belongs to the catalogue's choice of map, which may be another one.
    if let Some(entry) = env.args.as_ref().and_then(|a| a.0.entry.clone()) {
        setup.selected_entry_point = entry;
    }
    let session = &mut env.session;
    session.map_name = setup.selected_map.clone();
    session.entry_point = setup.selected_entry_point.clone();
    session.weather_preset = setup.weather_preset.clone();
    session.start_hour = setup.hour;
    session.start_minute = setup.minute;
    // The vehicle line of the loading screen was set once, when the session was entered
    // (`in_game::setup_in_game_session`); it is not touched again, so it never changes while loading.

    let args = env.args.as_ref().map(|a| &a.0);
    // An explicit `--vehicle-path` wins over *Drive without a vehicle*.
    let no_vehicle = setup.no_vehicle && args.and_then(|a| a.vehicle_path.as_deref()).is_none();
    let resolved = resolve_session_config(
        SessionRequest {
            map_path: args.and_then(|a| a.map_path.as_deref()),
            omsi_path: args.and_then(|a| a.omsi_path.as_deref()),
            vehicle_path: args.and_then(|a| a.vehicle_path.as_deref()),
            selected_map: &setup.selected_map,
            selected_vehicle: &setup.selected_vehicle,
            selected_entry_point: &setup.selected_entry_point,
            selected_livery: args
                .and_then(|a| a.livery.as_deref())
                .unwrap_or(&setup.selected_livery),
            no_vehicle,
        },
        env.catalogue.as_deref(),
        env.service.as_deref(),
    )
    .map(|mut cfg| {
        // A trip given by the protocol names its run only: its stop and texts come from the map's
        // timetables. A trip that does not resolve is dropped.
        if setup.trip.as_ref().is_some_and(|t| t.station_id == 0)
            && !setup
                .trip
                .as_mut()
                .is_some_and(|t| crate::trip_start::complete_trip(t, &cfg.map_dir))
        {
            tracing::warn!(trip = ?setup.trip, "the run is not in the map's timetables; no trip");
            setup.trip = None;
        }
        cfg.start_station = setup.trip.as_ref().map(|t| t.station_id);
        cfg
    })
    .and_then(|cfg| {
        // The map is built for the planned date (Chrono); the session environment starts there.
        let date = chrono_date(crate::new_game_screen::planner::setup_date(&setup));
        begin_world_load_at(cfg, Some(date)).map(|mut job| {
            job.no_vehicle = no_vehicle;
            // The share of the parking bays that hold a vehicle (setting *Parked vehicles*).
            job.parked_permille = env
                .settings
                .as_deref()
                .and_then(|s| s.raw("gameplay.parked")?.parse::<u32>().ok())
                .map_or(1000, |p| p.min(100) * 10);
            // *Load all tiles* and the tile radius decide what the loading screen builds (M-0080).
            let choice = env
                .settings
                .as_deref()
                .map(tile_loading::TileChoice::from_settings)
                .unwrap_or_default();
            let profile = env.hardware.as_deref().and_then(|h| h.profile.as_ref());
            job.queue_for_mode(tile_loading::resolve(choice, profile, job.map_radius()));
            (job.map_name(), job)
        })
    });
    // A chosen trip starts at its first stop (M-0145) and is followed once the vehicle is there.
    if let Some(t) = &setup.trip {
        env.session.entry_point.clone_from(&t.stop_name);
    }
    env.commands
        .insert_resource(crate::trip_start::PendingTrip(setup.trip.clone()));
    let overlay = env.stats.overlay_visible;
    *env.stats = WorldStats {
        overlay_visible: overlay,
        ..default()
    };
    match resolved {
        Ok((name, job)) => {
            env.stats.map_name = name.clone();
            env.stats.tiles_missing = job.tiles_listed_missing();
            env.stats.start_tile = job.start_tile();
            env.stats.grid = job.grid();
            // The card shows the map that is really loading, not the one the menu chose (F17).
            env.session.map_name = name;
            if !no_vehicle && !job.has_vehicle_file() && !setup.selected_vehicle.is_empty() {
                env.report.add(
                    LoadProblem::new(
                        false,
                        "load-problem-no-model",
                        &setup.selected_vehicle,
                        "the catalogue knows no .bus file for this vehicle",
                    )
                    .because("load-cause-no-vehicle-file"),
                );
            }
            env.commands.insert_resource(job.chrono_state());
            env.commands.insert_resource(job);
            state.progress = P_TERRAIN;
            state.stage = LoadStage::BuildTerrain;
        }
        Err(e) => {
            tracing::error!("cannot load OMSI world: {e}");
            env.report.add(
                LoadProblem::new(true, "load-problem-map", &setup.selected_map, e)
                    .because("load-cause-map-unreadable"),
            );
            fail(env, state);
        }
    }
}
