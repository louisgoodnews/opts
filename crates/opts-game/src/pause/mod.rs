//! The pause overlay and the way out of a drive
//! (`docs/src/roadmap/phase-9/screens.md` sections 4 and 5).
//!
//! Esc in a drive opens the overlay, pauses the simulation and releases the cursor; it never
//! leaves the session. *Leave drive…* asks once, naming the consequence, and offers *Leave to
//! Home* or *Quit OPTS*. Leaving a drive that moved shows the session summary on Home.

pub mod stats;

use bevy::app::AppExit;
use bevy::prelude::*;
use opts_bevy::reporting::{request_report, ReportModalData, ReportModalState};
use opts_gui::components::modal::{ModalBuilder, ModalEvent, ModalRoot};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::not_yet::{Availability, PAUSE_SETTINGS};
use opts_gui::prelude::*;

pub use stats::SessionStats;

use crate::app_state::AppState;
use crate::in_game::types::SimulationLoadState;
use crate::in_game::VehicleMenu;
use crate::new_game_screen::types::NewGameSetup;
use crate::session_summary::LastSession;
use crate::shell::dialog::{
    spawn_confirm_dialog, DialogButtonSpec, DialogButtonStyle, DialogChoice, DialogSpec, CANCEL,
};
use crate::shell::esc::EscGate;
use crate::shell::keys;

/// Tag of the leave confirmation.
pub const LEAVE_DIALOG: &str = "pause.leave";
const HOME: &str = "home";
const QUIT: &str = "quit";

/// State of the pause overlay.
#[derive(Resource, Debug, Default)]
pub struct Pause {
    /// The overlay is open.
    pub open: bool,
    /// The overlay's backdrop.
    pub root: Option<Entity>,
    /// The simulation was running when the overlay opened (so closing resumes it).
    pub resume_time: bool,
    /// The report dialog was opened from here: come back when it closes.
    pub after_report: bool,
    seen_report: bool,
}

/// Request to open the overlay (the HUD button, a gamepad's Start).
#[derive(Message, Debug, Clone, Copy)]
pub struct PauseRequest;

/// An entry of the overlay.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseItem {
    /// Close the overlay.
    Resume,
    /// Open the vehicle menu.
    Vehicle,
    /// Settings (not available yet during a drive).
    Settings,
    /// Controls (Help).
    Controls,
    /// The report dialog.
    Report,
    /// *Leave drive…*
    Leave,
}

fn item(
    column: &mut ChildSpawnerCommands,
    which: PauseItem,
    key: &'static str,
    id: &str,
) -> Entity {
    let button = ButtonBuilder::text(key).with_key(key).spawn_child(column);
    column.commands().entity(button).insert((
        which,
        UiId::new(format!("pause.{id}")),
        Node {
            min_height: Val::Px(44.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            ..default()
        },
    ));
    button
}

pub(crate) fn spawn_overlay(commands: &mut Commands) -> Entity {
    let (root, card) = ModalBuilder::new()
        .with_size_percent(30.0, 50.0)
        .with_width_range(320.0, 420.0)
        .spawn(commands);
    commands.queue(move |world: &mut World| {
        if let Some(mut node) = world.entity_mut(card).get_mut::<Node>() {
            node.height = Val::Auto;
        }
    });
    commands.entity(card).with_children(|c| {
        c.spawn((
            Text::new(keys::PAUSE_TITLE),
            L10nKey(keys::PAUSE_TITLE),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        let resume = item(c, PauseItem::Resume, keys::PAUSE_RESUME, "resume");
        c.commands().entity(resume).insert(DefaultFocus);
        item(c, PauseItem::Vehicle, keys::PAUSE_VEHICLE, "vehicle");
        let settings = item(c, PauseItem::Settings, keys::PAUSE_SETTINGS, "settings");
        Availability::NotYet(&PAUSE_SETTINGS).apply(&mut c.commands().entity(settings));
        item(c, PauseItem::Controls, keys::PAUSE_CONTROLS, "controls");
        item(c, PauseItem::Report, keys::PAUSE_REPORT, "report");
        c.spawn(Node {
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        })
        .insert(BackgroundRole(ColorRole::OutlineVariant));
        let leave = item(c, PauseItem::Leave, keys::PAUSE_LEAVE, "leave");
        c.commands().entity(leave).insert((
            BackgroundRole(ColorRole::ErrorContainer),
            StateLayerRole(ColorRole::OnErrorContainer),
        ));
    });
    commands.entity(root).insert(Name::new("PauseOverlay"));
    root
}

fn open_pause(commands: &mut Commands, pause: &mut Pause, time: &mut Time<Virtual>) {
    if pause.open {
        return;
    }
    pause.resume_time = !time.is_paused();
    time.pause();
    pause.open = true;
    pause.root = Some(spawn_overlay(commands));
}

fn close_pause(commands: &mut Commands, pause: &mut Pause, time: &mut Time<Virtual>, resume: bool) {
    if let Some(root) = pause.root.take() {
        commands.entity(root).try_despawn();
    }
    pause.open = false;
    if resume && pause.resume_time {
        time.unpause();
    }
}

/// Esc (or Start) in a running drive opens the overlay; during loading Esc cancels the load.
#[allow(clippy::too_many_arguments)]
pub fn pause_input(
    mut commands: Commands,
    mut actions: MessageReader<UiAction>,
    mut requests: MessageReader<PauseRequest>,
    gate: Res<EscGate>,
    keys: Res<ButtonInput<KeyCode>>,
    load: Res<SimulationLoadState>,
    mut pause: ResMut<Pause>,
    mut time: ResMut<Time<Virtual>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let mut open = requests.read().count() > 0;
    for action in actions.read() {
        match action {
            // Backspace is also "Back" for the UI, but a drive uses it for the vehicle.
            UiAction::Back
                if !gate.closed()
                    && (keys.just_pressed(KeyCode::Escape)
                        || !keys.just_pressed(KeyCode::Backspace)) =>
            {
                if load.is_complete {
                    open = true;
                } else {
                    tracing::info!("Back while loading: cancelling the load");
                    next.set(AppState::MainMenu);
                }
            }
            UiAction::Menu if !gate.closed() && load.is_complete => open = true,
            _ => {}
        }
    }
    if open && load.is_complete {
        open_pause(&mut commands, &mut pause, &mut time);
    }
}

/// Esc inside the overlay resumes.
pub fn pause_cancel(
    mut commands: Commands,
    mut events: MessageReader<ModalEvent>,
    roots: Query<(), With<ModalRoot>>,
    mut pause: ResMut<Pause>,
    mut time: ResMut<Time<Virtual>>,
) {
    for ModalEvent::CancelRequested(modal) in events.read() {
        if pause.root == Some(*modal) && roots.contains(*modal) {
            close_pause(&mut commands, &mut pause, &mut time, true);
        }
    }
}

/// The overlay's entries.
#[allow(clippy::too_many_arguments)]
pub fn pause_items(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    items: Query<&PauseItem>,
    disabled: Query<(), With<opts_gui::interaction::state::Disabled>>,
    mut pause: ResMut<Pause>,
    mut time: ResMut<Time<Virtual>>,
    mut menu: ResMut<VehicleMenu>,
    mut report: ResMut<ReportModalData>,
    report_state: Res<State<ReportModalState>>,
    mut report_next: ResMut<NextState<ReportModalState>>,
    session: Res<SessionStats>,
) {
    for Activated(entity) in activated.read() {
        let Ok(which) = items.get(*entity) else {
            continue;
        };
        if disabled.contains(*entity) {
            continue;
        }
        match which {
            PauseItem::Resume => close_pause(&mut commands, &mut pause, &mut time, true),
            PauseItem::Vehicle => {
                close_pause(&mut commands, &mut pause, &mut time, true);
                menu.open_main();
            }
            PauseItem::Report => {
                // The report takes its screenshot of the game, not of the overlay; the overlay
                // comes back when the dialog closes and the simulation stays paused.
                close_pause(&mut commands, &mut pause, &mut time, false);
                pause.after_report = true;
                pause.seen_report = false;
                request_report(&time, &mut report, &report_state, &mut report_next);
            }
            PauseItem::Leave => {
                let moved = session.moved();
                if moved {
                    // One dialog at a time: the overlay steps aside and returns on Cancel.
                    if let Some(root) = pause.root.take() {
                        commands.entity(root).try_despawn();
                    }
                    spawn_confirm_dialog(&mut commands, leave_spec());
                } else {
                    // Nothing would be lost: no question.
                    commands.queue(|world: &mut World| leave_to_home(world));
                }
            }
            PauseItem::Controls => {
                // One dialog at a time: the overlay steps aside and returns when Help closes.
                if let Some(root) = pause.root.take() {
                    commands.entity(root).try_despawn();
                }
                crate::help::open_overlay(&mut commands);
            }
            PauseItem::Settings => {}
        }
    }
}

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
    mut time: ResMut<Time<Virtual>>,
) {
    if let Some(root) = pause.root.take() {
        commands.entity(root).try_despawn();
    }
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

/// Registers the overlay.
pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        crate::pause::ensure_report_state(app);
        stats::build(app);
        app.init_resource::<Pause>()
            .add_message::<PauseRequest>()
            .add_systems(
                Update,
                (
                    pause_input.after(opts_gui::actions::ActionInput),
                    pause_cancel,
                    pause_items,
                    leave_choices,
                    reopen_after_report,
                )
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnExit(AppState::InGame), reset_on_exit);
    }
}

#[cfg(test)]
mod tests;
