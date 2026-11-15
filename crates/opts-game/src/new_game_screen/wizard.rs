//! The wizard's lifecycle: entering, building and rebuilding the step body, and leaving.

use super::*;

/// Entering the wizard: first step, the last drive's choices (once per session), the frame.
pub(super) fn enter_wizard(
    mut commands: Commands,
    shell: Res<ShellContent>,
    read: WizardRead,
    mut setup: ResMut<NewGameSetup>,
    mut history: ResMut<PlannerHistory>,
    mut state: ResMut<PlannerState>,
    mut events: MessageWriter<FocusEvent>,
) {
    let WizardRead {
        loc,
        catalogue,
        weather,
        trips,
    } = read;
    let notes = history.apply_last(&mut setup, &catalogue);
    // A remembered drive brings its date and time: choosing a map keeps them.
    if history.history.last().is_some() && !catalogue.maps.is_empty() {
        state.date_chosen = true;
        state.time_chosen = true;
    }
    planner::ensure_valid(&mut setup, &catalogue);
    state.step = PlannerStep::Where;
    state.built = Some(PlannerStep::Where);
    state.notes = notes;
    let (_, body) = screen::spawn_frame(&mut commands, &shell);
    let ctx = BuildCtx {
        loc: &loc,
        catalogue: &catalogue,
        weather: &weather,
        setup: &setup,
        history: &history,
        notes: &state.notes,
        trips: &trips,
    };
    let built = build_step(&mut commands, &ctx, state.step);
    commands.entity(body).add_child(built.0);
    if let Some(f) = built.1 {
        events.write(FocusEvent::Set(f));
    }
}

/// Builds the body of `step`; returns its root and the control that should take focus.
pub(super) fn build_step(
    commands: &mut Commands,
    ctx: &BuildCtx,
    step: PlannerStep,
) -> (Entity, Option<Entity>) {
    match step {
        PlannerStep::Where => {
            let root = step_where::spawn_where(commands, ctx);
            (root, None)
        }
        PlannerStep::What => {
            let root = step_what::spawn_what(commands, ctx);
            (root, None)
        }
        PlannerStep::When => (step_when::spawn_when(commands, ctx), None),
        PlannerStep::Trip => (trips::spawn_trip(commands, ctx), None),
        PlannerStep::Review => (step_review::spawn_review(commands, ctx), None),
    }
}

/// A step change replaces the body.
pub(super) fn rebuild_body(
    mut state: ResMut<PlannerState>,
    body: Query<Entity, With<PlannerBody>>,
    read: WizardRead,
    setup: Res<NewGameSetup>,
    history: Res<PlannerHistory>,
    mut commands: Commands,
    mut built_with: Local<(usize, usize, usize)>,
) {
    let WizardRead {
        loc,
        catalogue,
        weather,
        trips,
    } = read;
    let Ok(body) = body.single() else {
        return;
    };
    // The catalogue may be (re)read while the wizard is open (the first scan of a fresh
    // installation takes seconds): a step built from an empty or older list is built again.
    let content = (
        catalogue.maps.len(),
        catalogue.vehicles.len(),
        weather.ids().len(),
    );
    let content_changed = *built_with != content;
    *built_with = content;
    if state.built == Some(state.step) && !content_changed {
        return;
    }
    state.built = Some(state.step);
    commands.entity(body).despawn_related::<Children>();
    let ctx = BuildCtx {
        loc: &loc,
        catalogue: &catalogue,
        weather: &weather,
        setup: &setup,
        history: &history,
        notes: &state.notes,
        trips: &trips,
    };
    let (root, _) = build_step(&mut commands, &ctx, state.step);
    commands.entity(body).add_child(root);
}

/// Leaving the wizard removes its screen and dialogs.
pub(super) fn cleanup_wizard(
    mut commands: Commands,
    roots: Query<Entity, With<NewGameScreenRoot>>,
    modals: Query<Entity, With<options::OptionsModal>>,
) {
    for e in roots.iter().chain(modals.iter()) {
        commands.entity(e).despawn();
    }
}

/// Leaving the wizard: *Back* on the next screen has no step of this one to take.
pub(super) fn reset_back_steps(mut steps: ResMut<crate::shell::esc::ScreenBackSteps>) {
    steps.0 = false;
}
