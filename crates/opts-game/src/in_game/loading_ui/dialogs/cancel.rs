//! The *Cancel loading?* dialog: its opening, the buttons of the dialog, and `Esc`.

use super::super::outcome::outcome_button;
use super::super::*;

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
