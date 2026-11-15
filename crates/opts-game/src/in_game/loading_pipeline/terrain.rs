//! The terrain and atmosphere stages of the loading pipeline: the map's tiles and the sky.

use super::*;

pub(super) fn stage_terrain(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    let LoadEnv {
        commands,
        assets,
        job,
        stats,
        ..
    } = env;
    let Some(job) = job.as_deref_mut() else {
        state.stage = LoadStage::SetupAtmosphere;
        return;
    };
    if job.has_tile_work() {
        // A budget per frame, not a whole tile: the window keeps answering while a dense tile
        // builds (M-0089).
        let mut budget = FrameBudget::new(tile_loading::LOADING_BUDGET);
        advance_tile_loading(
            (job, stats, commands),
            (
                &mut assets.meshes,
                &mut assets.materials,
                &mut assets.images,
            ),
            assets.ground_materials.as_deref_mut(),
            (&mut budget, usize::MAX),
        );
    }
    (state.tiles_done, state.tiles_total) = job.tile_counts();
    state.progress = P_TERRAIN + P_TERRAIN_SPAN * job.progress();
    if job.tiles_done() {
        state.world_spawned = true;
        if env.stats.tiles_loaded == 0 {
            let name = env.stats.map_name.clone();
            env.report.add(
                LoadProblem::new(
                    true,
                    "load-problem-map",
                    &name,
                    "no map tile could be loaded",
                )
                .because("load-cause-no-tiles"),
            );
            fail(env, state);
            return;
        }
        state.stage = LoadStage::SetupAtmosphere;
    }
}

pub(super) fn stage_atmosphere(env: &mut LoadEnv, state: &mut SimulationLoadState) {
    if !state.atmosphere_spawned {
        let LoadEnv {
            commands,
            assets,
            job,
            ..
        } = env;
        let sky = job.as_deref().map(|j| {
            super::super::sky::spawn_sky(
                commands,
                &mut assets.meshes,
                &mut assets.materials,
                &mut assets.images,
                j.omsi_root(),
            )
        });
        match sky {
            Some(Ok(())) => {}
            other => {
                if let Some(Err(e)) = other {
                    tracing::error!(error = %e, "sky unavailable; using default sun light");
                }
                spawn_atmosphere_and_lighting(commands);
            }
        }
        state.atmosphere_spawned = true;
    }
    state.progress = P_LIGHT;
    state.stage = LoadStage::SpawnVehicle;
}
