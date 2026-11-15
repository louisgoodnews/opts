//! Moving through the wizard: Back, Next, the tabs, the `Esc` back step and starting the drive.

use bevy::prelude::*;
use opts_gui::actions::UiAction;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::new_game_screen::planner::{PlannerState, validate};
use crate::new_game_screen::screen::step_invalid;
use crate::shell::esc::{ScreenBack, ScreenBackSteps};
use crate::shell::nav::NavigateBack;

use super::{Env, now_unix};

pub(super) fn back(env: &mut Env) {
    match env.state.prev_step() {
        Some(prev) => env.state.step = prev,
        None => {
            env.leave.write(NavigateBack);
        }
    }
}

pub(super) fn next(env: &mut Env) {
    if step_invalid(env.state.step, &env.setup, &env.catalogue).is_some() {
        return;
    }
    if let Some(n) = env.state.next_step() {
        env.state.step = n;
    }
}

pub(super) fn drive(env: &mut Env, commands: &mut Commands) {
    if validate(&env.setup, &env.catalogue).is_err() {
        return;
    }
    let setup = env.setup.clone();
    if let Err(e) = env.history.record_drive(&setup, now_unix()) {
        tracing::warn!("cannot save the drive history: {e}");
        let path = env
            .history
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        ToastBuilder::new(
            env.loc
                .t_args("wiz-save-failed", &[("path".to_string(), path.into())]),
        )
        .with_duration(6.0)
        .spawn(commands);
    }
    tracing::info!(
        map = %setup.selected_map,
        vehicle = %setup.selected_vehicle,
        "starting the drive"
    );
    // A composed train holds only for the rail vehicle it was made for (the player may have gone
    // back and chosen another one: choosing clears it).
    let rail = !setup.no_vehicle
        && env
            .catalogue
            .find_vehicle(&setup.selected_vehicle)
            .is_some_and(|v| v.class == "rail");
    if !rail {
        env.setup.formation.clear();
    }
    env.next_state.set(AppState::InGame);
}

/// `Esc` / `Back` inside the wizard: the shell's one Esc mechanism (`shell::esc`) sends
/// [`ScreenBack`] while a step is left to go back to, and leaves the screen through the back stack
/// otherwise. Dialogs, open drop-downs and text fields keep the press (the shell's gate).
pub fn handle_back_action(mut requests: MessageReader<ScreenBack>, mut env: Env) {
    for _ in requests.read() {
        back(&mut env);
    }
}

/// Tells the shell whether *Back* has a step to go back to inside the wizard.
pub fn publish_back_steps(state: Res<PlannerState>, mut steps: ResMut<ScreenBackSteps>) {
    steps.set_if_neq(ScreenBackSteps(state.prev_step().is_some()));
}

/// `]` and `[` switch steps like tabs.
pub fn handle_tab_actions(mut actions: MessageReader<UiAction>, mut env: Env) {
    for action in actions.read() {
        match action {
            UiAction::NextTab => next(&mut env),
            UiAction::PrevTab => {
                if let Some(p) = env.state.prev_step() {
                    env.state.step = p;
                }
            }
            _ => {}
        }
    }
}
