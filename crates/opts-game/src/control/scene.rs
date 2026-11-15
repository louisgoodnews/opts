//! Scene commands of the control protocol: state, scene, the restart waiter and its session.


use super::*;


pub(super) fn scene(world: &mut World, _: &Value) -> Outcome {
    let Some(stats) = world.get_resource::<WorldStats>().cloned() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    };
    let meshes = world.resource::<Assets<Mesh>>().len();
    let materials = world.resource::<Assets<StandardMaterial>>().len();
    let images = world.resource::<Assets<Image>>().len();
    let live_entities = world.query::<Entity>().iter(world).count();
    Outcome::ok(json!({
        "map": stats.map_name,
        "start_tile": [stats.start_tile.0, stats.start_tile.1],
        "tiles_loaded": stats.tiles_loaded,
        "tiles_missing": stats.tiles_missing,
        "entity_count": world.entities().len(),
        "entities_live": live_entities,
        "meshes": meshes,
        "materials": materials,
        "images": images,
        "asset_count": meshes + materials + images,
        "stats": {
            "objects_loaded": stats.objects_loaded,
            "objects_failed": stats.objects_failed,
            "objects_protected": stats.objects_protected,
            "trees": stats.trees_loaded,
            "splines_loaded": stats.splines_loaded,
            "splines_failed": stats.splines_failed,
            "scene_vertices": stats.scene_vertices,
            "asset_failures": stats.asset_failures,
            "vehicle": stats.vehicle_name,
        },
    }))
}

/// A waiter that drives the app through leaving the current session and entering a fresh one; it
/// finishes with a summary once the new session's HUD is up.
pub(crate) fn restart_waiter(world: &mut World) -> Waiter {
    let was_in_game = app_state(world) == AppState::InGame;
    world
        .resource_mut::<NextState<AppState>>()
        .set(if was_in_game {
            AppState::MainMenu
        } else {
            AppState::InGame
        });
    let mut left = !was_in_game;
    Box::new(move |world| {
        if !left {
            if app_state(world) != AppState::MainMenu {
                return None;
            }
            left = true;
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::InGame);
            return None;
        }
        if app_state(world) != AppState::InGame {
            return None;
        }
        // A load that failed answers at once instead of timing out; a load with problems answers
        // while the loading screen waits for *Continue* (the session is usable, `loading.continue`
        // starts it), so a script is never blocked on a person's click.
        if let Some(report) = world.get_resource::<crate::in_game::load_report::LoadReport>() {
            use crate::in_game::load_report::LoadOutcome;
            let details: Vec<String> = report.problems.iter().map(|p| p.detail.clone()).collect();
            match report.outcome {
                LoadOutcome::Failed => {
                    return Some(Err(RpcError::new(
                        code::FAILED,
                        format!("the load failed: {}", details.join("; ")),
                    )));
                }
                LoadOutcome::ReadyWithProblems => {
                    if crate::control_loading::accepted_and_starting(world, report) {
                        return None;
                    }
                    let stats = world.resource::<WorldStats>();
                    return Some(Ok(json!({
                        "map": stats.map_name, "vehicle": stats.vehicle_name,
                        "tiles_loaded": stats.tiles_loaded,
                        "waiting_for_continue": !world
                            .get_resource::<SimulationLoadState>()
                            .is_some_and(|s| s.hud_spawned),
                        "problems": details,
                    })));
                }
                _ => {}
            }
        }
        let ready = world
            .get_resource::<SimulationLoadState>()
            .is_some_and(|s| s.hud_spawned);
        ready.then(|| {
            let stats = world.resource::<WorldStats>();
            Ok(json!({ "map": stats.map_name, "vehicle": stats.vehicle_name, "tiles_loaded": stats.tiles_loaded }))
        })
    })
}

/// Brings the app into a fresh in-game session, leaving the current one first. Answers when the
/// HUD of the new session is up.
pub(super) fn restart_session(world: &mut World) -> Outcome {
    Outcome::Wait(restart_waiter(world))
}
