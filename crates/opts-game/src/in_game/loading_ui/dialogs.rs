//! The cancel dialog and the minor-problem dialog of the loading screen.

use super::outcome::{outcome_button, problem_row};
use super::*;

/// Opens the *Cancel loading?* dialog (default focus on *Keep loading*).
pub fn ask_to_cancel(commands: &mut Commands) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 30.0)
        .with_width_range(360.0, 520.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((CancelDialog, UiId::new("loading.cancel-dialog")));
    let title = commands
        .spawn(keyed(
            "load-cancel-title",
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let body = commands
        .spawn(keyed(
            "load-cancel-body",
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let line = commands
        .spawn(Node {
            justify_content: JustifyContent::FlexEnd,
            margin: UiRect::top(Val::Auto),
            ..row(12.0)
        })
        .id();
    let yes = outcome_button(commands, "load-cancel-yes", false, "loading.cancel-yes");
    commands.entity(yes).insert(ConfirmCancelBtn);
    let keep = outcome_button(commands, "load-cancel-keep", true, "loading.cancel-keep");
    commands.entity(keep).insert((KeepLoadingBtn, DefaultFocus));
    commands.entity(line).add_children(&[yes, keep]);
    commands.entity(content).add_children(&[title, body, line]);
}

/// The queries of [`handle_cancel`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct CancelQueries<'w, 's> {
    cancel: Query<'w, 's, (), With<CancelLoadingBtn>>,
    keep: Query<'w, 's, (), With<KeepLoadingBtn>>,
    confirm: Query<'w, 's, (), With<ConfirmCancelBtn>>,
    dialogs: Query<'w, 's, Entity, With<CancelDialog>>,
    other_modals: Query<'w, 's, (), (With<ModalRoot>, Without<CancelDialog>)>,
    screen: Query<'w, 's, (), With<LoadingScreenRoot>>,
}

/// The cancel button, the dialog's buttons and `Esc`.
pub fn handle_cancel(
    mut activated: MessageReader<Activated>,
    mut actions: MessageReader<UiAction>,
    mut modal_events: MessageReader<ModalEvent>,
    queries: CancelQueries,
    report: Res<LoadReport>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    let CancelQueries {
        cancel,
        keep,
        confirm,
        dialogs,
        other_modals,
        screen,
    } = queries;
    if screen.is_empty() {
        return;
    }
    for Activated(e) in activated.read() {
        if cancel.contains(*e) && dialogs.is_empty() {
            ask_to_cancel(&mut commands);
        } else if keep.contains(*e) {
            close(&mut commands, &dialogs);
        } else if confirm.contains(*e) {
            close(&mut commands, &dialogs);
            next_state.set(AppState::VehicleSelection);
        }
    }
    for ev in modal_events.read() {
        let ModalEvent::CancelRequested(m) = ev;
        if dialogs.contains(*m) {
            close(&mut commands, &dialogs);
        }
    }
    for a in actions.read() {
        if *a == UiAction::Back && dialogs.is_empty() && other_modals.is_empty() {
            if report.outcome == LoadOutcome::Failed {
                next_state.set(AppState::VehicleSelection);
            } else {
                ask_to_cancel(&mut commands);
            }
        }
    }
}

fn close(commands: &mut Commands, dialogs: &Query<Entity, With<CancelDialog>>) {
    for d in dialogs {
        commands.entity(d).despawn();
    }
}

/// The dialog that lists the minor problems.
#[derive(Component, Debug, Clone, Copy)]
pub struct MinorDialog;

/// *Close* in that dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct MinorCloseBtn;

/// The toast that announced the minor problems (its action opens [`MinorDialog`]).
#[derive(Resource, Debug, Clone, Copy)]
pub struct MinorToast(pub Entity);

/// After the first HUD frame: one non-blocking toast when only minor problems were found.
pub fn announce_minor_problems(
    state: Res<SimulationLoadState>,
    mut report: ResMut<LoadReport>,
    loc: Res<Localization>,
    mut commands: Commands,
) {
    if !state.hud_spawned || report.minor_announced || report.minor_count() == 0 {
        return;
    }
    report.minor_announced = true;
    let n = report.minor_count();
    let args = [("count".to_string(), n.into())]; // l10n-ok: variable name
    let toast = ToastBuilder::new(loc.t_args("load-minor-toast", &args))
        .with_duration(12.0)
        .with_action(loc.t("load-details"))
        .spawn(&mut commands);
    commands.insert_resource(MinorToast(toast));
}

/// The queries of [`handle_minor_dialog`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct MinorQueries<'w, 's> {
    actions: Query<'w, 's, &'static opts_gui::components::toast::ToastActionButton>,
    close: Query<'w, 's, (), With<MinorCloseBtn>>,
    dialogs: Query<'w, 's, Entity, With<MinorDialog>>,
}

/// The toast's *Details* opens the list; *Close* and `Esc` close it.
pub fn handle_minor_dialog(
    mut activated: MessageReader<Activated>,
    mut modal_events: MessageReader<ModalEvent>,
    queries: MinorQueries,
    toast: Option<Res<MinorToast>>,
    report: Res<LoadReport>,
    mut commands: Commands,
) {
    let MinorQueries {
        actions,
        close,
        dialogs,
    } = queries;
    for Activated(e) in activated.read() {
        let opens = actions
            .get(*e)
            .is_ok_and(|a| toast.as_ref().is_some_and(|t| t.0 == a.0));
        if opens && dialogs.is_empty() {
            open_minor_dialog(&mut commands, &report);
        } else if close.contains(*e) {
            for d in &dialogs {
                commands.entity(d).despawn();
            }
        }
    }
    for ModalEvent::CancelRequested(m) in modal_events.read() {
        if dialogs.contains(*m) {
            commands.entity(*m).despawn();
        }
    }
}

fn open_minor_dialog(commands: &mut Commands, report: &LoadReport) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(50.0, 60.0)
        .with_width_range(400.0, 720.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((MinorDialog, UiId::new("loading.minor-dialog")));
    let title = commands
        .spawn(keyed(
            "load-minor-title",
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let list = commands
        .spawn(Node {
            flex_grow: 1.0,
            overflow: Overflow::scroll_y(),
            ..column(8.0)
        })
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, list);
    let rows: Vec<Entity> = report
        .problems
        .iter()
        .filter(|p| p.severity() == Severity::Minor)
        .map(|p| problem_row(commands, p, true))
        .collect();
    commands.entity(list).add_children(&rows);
    let close = outcome_button(commands, "load-close", true, "loading.minor-close");
    commands.entity(close).insert((MinorCloseBtn, DefaultFocus));
    commands.entity(content).add_children(&[title, list, close]);
}
