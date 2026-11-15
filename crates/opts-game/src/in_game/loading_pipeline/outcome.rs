//! The ready stage and the outcome of a load: the buttons, the restart and the hand-over to the
//! active simulation.

use super::*;

pub(super) fn stage_ready(
    env: &mut LoadEnv,
    state: &mut SimulationLoadState,
    dt: f32,
    theme: &MaterialTheme,
    loading_screen: &Query<Entity, With<LoadingScreenRoot>>,
) {
    let go = match env.report.outcome {
        LoadOutcome::Ready => {
            state.auto_launch_delay -= dt;
            state.auto_launch_delay <= 0.0
        }
        LoadOutcome::ReadyWithProblems => {
            let by_flag = env.args.as_ref().is_some_and(|a| a.0.accept_load_problems);
            let always = env.accept_always.as_ref().is_some_and(|a| a.0);
            let accepted = by_flag || env.accept.0 || always;
            if accepted && !env.report.continue_requested {
                tracing::warn!(
                    problems = env.report.blocking_count(),
                    "starting the drive although the load had problems (accepted):\n{}",
                    env.report.as_text()
                );
            }
            if accepted {
                // The protocol's one-load acceptance is used up; the load counts as continued.
                env.accept.0 = false;
                env.report.continue_requested = true;
            }
            env.report.continue_requested || accepted
        }
        LoadOutcome::Loading | LoadOutcome::Failed => false,
    };
    if go {
        transition_to_active_simulation(
            &mut env.commands,
            state,
            &env.session,
            theme,
            loading_screen,
        );
    }
}

/// Which outcome button an entity is.
type OutcomeButton = (
    Has<DetailsBtn>,
    Has<ContinueBtn>,
    Has<RetryBtn>,
    Has<ChooseAnotherBtn>,
    Has<LoadingBackBtn>,
);

/// Handles the buttons of the loading screen: cancel, details, continue, retry, choose another,
/// back.
pub fn handle_loading_interactions(
    mut activated: MessageReader<Activated>,
    cancel: Query<(), With<CancelLoadingBtn>>,
    buttons: Query<OutcomeButton>,
    mut report: ResMut<LoadReport>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for Activated(e) in activated.read() {
        if cancel.contains(*e) {
            // The dialog asks first; see `loading_ui::ask_to_cancel`.
            continue;
        }
        let Ok((details, cont, retry, another, back)) = buttons.get(*e) else {
            continue;
        };
        if details {
            report.details_open = !report.details_open;
        } else if cont {
            report.continue_requested = true;
        } else if retry {
            report.retry_requested = true;
        } else if another {
            next_state.set(AppState::VehicleSelection);
        } else if back {
            next_state.set(AppState::MainMenu);
        }
    }
}

/// Starts the load again on the same screen: the half-built world goes, the report is cleared.
pub(super) fn restart(
    env: &mut LoadEnv,
    state: &mut SimulationLoadState,
    session_entities: &Query<Entity, (With<InGameEntity>, Without<LoadingScreenRoot>)>,
) {
    for e in session_entities {
        env.commands.entity(e).despawn();
    }
    env.commands.remove_resource::<WorldLoadJob>();
    env.commands.remove_resource::<super::super::sky::Sky>();
    env.registry.clear();
    *state = SimulationLoadState::default();
    env.report.reset();
}

/// Transitions from the loading screen to active gameplay: despawns loading overlay and spawns the driving HUD.
fn transition_to_active_simulation(
    commands: &mut Commands,
    state: &mut SimulationLoadState,
    session: &InGameSessionInfo,
    theme: &MaterialTheme,
    loading_screen_query: &Query<Entity, With<LoadingScreenRoot>>,
) {
    for entity in loading_screen_query {
        commands.entity(entity).despawn();
    }
    if !state.hud_spawned {
        spawn_in_game_hud(commands, theme, session);
        state.hud_spawned = true;
        let mips = opts_bevy::texture::mip_stats();
        tracing::info!(
            textures = mips.textures,
            millis = mips.micros / 1000,
            dropped_dds_chains = opts_bevy::texture::dropped_dds_chains(),
            "generated mip chains"
        );
        let bc = opts_bevy::texture::bc_chain_stats();
        tracing::info!(
            built = bc.built,
            from_cache = bc.cached,
            millis = bc.micros / 1000,
            "block-compressed mip chains"
        );
        tracing::info!("Simulation viewport active; driving HUD instantiated");
    }
}
