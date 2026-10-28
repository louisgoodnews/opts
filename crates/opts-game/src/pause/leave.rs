//! Leave confirmation dialog, overlay return, and exit cleanup for the pause overlay.

use super::*;

/// The leave confirmation: the consequence, then the way out. Cancel comes first so it has focus.
pub fn leave_spec() -> DialogSpec {
    DialogSpec {
        tag: LEAVE_DIALOG,
        title_key: keys::LEAVE_TITLE,
        body_key: keys::LEAVE_BODY,
        body_args: L10nArgs::default(),
        buttons: vec![
            DialogButtonSpec {
                choice: CANCEL,
                label_key: keys::CANCEL,
                style: DialogButtonStyle::Quiet,
            },
            DialogButtonSpec {
                choice: HOME,
                label_key: keys::LEAVE_HOME,
                style: DialogButtonStyle::Destructive,
            },
            DialogButtonSpec {
                choice: QUIT,
                label_key: keys::LEAVE_QUIT,
                style: DialogButtonStyle::Destructive,
            },
        ],
        keep_open: Vec::new(),
        error: None,
    }
}

/// Ends the drive: remembers it for the summary and returns to Home.
fn leave_to_home(world: &mut World) {
    let stats = world.resource::<SessionStats>().clone();
    let setup = world.resource::<NewGameSetup>().clone();
    if stats.moved() {
        world.insert_resource(LastSession::new(&setup, stats));
    }
    let root = world.resource_mut::<Pause>().root.take();
    if let Some(entity) = root.and_then(|r| world.get_entity_mut(r).ok()) {
        entity.despawn();
    }
    world.resource_mut::<Pause>().open = false;
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
}

/// The answer of the leave dialog.
pub fn leave_choices(
    mut commands: Commands,
    mut choices: MessageReader<DialogChoice>,
    mut exit: MessageWriter<AppExit>,
    mut pause: ResMut<Pause>,
) {
    for choice in choices.read().filter(|c| c.tag == LEAVE_DIALOG) {
        match choice.choice {
            HOME => {
                commands.queue(|world: &mut World| leave_to_home(world));
            }
            QUIT => {
                exit.write(AppExit::Success);
            }
            _ => {
                if pause.open && pause.root.is_none() {
                    pause.root = Some(spawn_overlay(&mut commands));
                }
            }
        }
    }
}

/// When the Help or Settings overlay closes during a drive, the pause overlay is back.
pub fn return_after_overlay(
    mut closed: RemovedComponents<crate::help::HelpOverlay>,
    mut settings_closed: RemovedComponents<SettingsOverlay>,
    mut saves_closed: RemovedComponents<crate::save::ui::SavesOverlay>,
    mut environment_closed: RemovedComponents<environment::EnvironmentOverlay>,
    mut pause: ResMut<Pause>,
    reporting: Res<State<ReportModalState>>,
    mut commands: Commands,
) {
    if closed.read().count()
        + settings_closed.read().count()
        + saves_closed.read().count()
        + environment_closed.read().count()
        == 0
    {
        return;
    }
    // The report dialog (opened from the overlay) brings the pause overlay back itself.
    if pause.open && pause.root.is_none() && *reporting.get() == ReportModalState::Inactive {
        pause.root = Some(spawn_overlay(&mut commands));
    }
}

/// After the report dialog closes the overlay returns.
pub fn reopen_after_report(
    mut commands: Commands,
    state: Res<State<ReportModalState>>,
    mut pause: ResMut<Pause>,
    mut time: ResMut<Time<Virtual>>,
) {
    if !pause.after_report {
        return;
    }
    if *state.get() != ReportModalState::Inactive {
        pause.seen_report = true;
    } else if pause.seen_report {
        pause.after_report = false;
        pause.seen_report = false;
        pause.resume_time = true;
        open_pause_again(&mut commands, &mut pause, &mut time);
    }
}

fn open_pause_again(commands: &mut Commands, pause: &mut Pause, time: &mut Time<Virtual>) {
    pause.open = false;
    let keep = pause.resume_time;
    open_pause(commands, pause, time);
    pause.resume_time = keep;
}

/// The HUD button and the cleanup when a drive ends.
pub fn reset_on_exit(
    mut commands: Commands,
    mut pause: ResMut<Pause>,
    mut page: ResMut<environment::EnvironmentPage>,
    pages: Query<Entity, With<environment::EnvironmentOverlay>>,
    mut time: ResMut<Time<Virtual>>,
) {
    if let Some(root) = pause.root.take() {
        commands.entity(root).try_despawn();
    }
    for root in &pages {
        commands.entity(root).try_despawn();
    }
    *page = environment::EnvironmentPage::default();
    *pause = Pause::default();
    time.unpause();
}

/// The report dialog's state exists even in headless runs (where the reporting plugin is absent).
pub fn ensure_report_state(app: &mut App) {
    if !app.world().contains_resource::<State<ReportModalState>>() {
        app.init_state::<ReportModalState>();
    }
    app.init_resource::<ReportModalData>();
}
