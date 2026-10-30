//! Esc and Back: one implementation for every screen of the shell
//! (`docs/src/roadmap/phase-9/interaction-and-feel.md` section 6).
//!
//! The library's `back_system` already closes an open combobox, dismisses a focused toast and
//! drops the focus of a text entry. A dialog or overlay closes itself on the modal cancel request.
//! The shell acts only when none of these was in play at the start of the frame, so one press
//! never does two things. In a drive the pause overlay owns Esc (the pause overlay).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::reporting::{MenuModalActive, ModalActive};
use opts_gui::components::combobox::M3Combobox;
use opts_gui::components::modal::ModalRoot;
use opts_gui::components::toast::ToastActionButton;
use opts_gui::interaction::handlers::TextEntry;
use opts_gui::interaction::state::Focused;
use opts_gui::prelude::UiAction;

use super::nav::NavigateBack;
use crate::app_state::AppState;

/// Whether something else owns this frame's Back press; computed before any system of the frame
/// can close it.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EscGate {
    /// A modal, the report dialog or the vehicle menu is open.
    pub overlay_open: bool,
    /// A combobox list is open.
    pub popup_open: bool,
    /// A text entry has the focus.
    pub text_focused: bool,
    /// A toast has the focus.
    pub toast_focused: bool,
    /// The viewpoint editor of the driver's view is open (Esc cancels it).
    pub viewpoint_open: bool,
}

impl EscGate {
    /// Whether the press belongs to something other than screen navigation.
    pub fn closed(&self) -> bool {
        self.overlay_open
            || self.popup_open
            || self.text_focused
            || self.toast_focused
            || self.viewpoint_open
    }
}

/// What can own the Back press besides the screen.
#[derive(SystemParam)]
pub struct EscProbes<'w, 's> {
    modals: Query<'w, 's, (), With<ModalRoot>>,
    combos: Query<'w, 's, &'static M3Combobox>,
    text: Query<'w, 's, (), (With<Focused>, TextEntry)>,
    toast: Query<'w, 's, (), (With<Focused>, With<ToastActionButton>)>,
}

/// Snapshots the state of the screen at the start of the frame.
pub fn compute_esc_gate(
    probes: EscProbes,
    report: Option<Res<ModalActive>>,
    menu: Option<Res<MenuModalActive>>,
    viewpoint: Option<Res<opts_bevy::ViewpointState>>,
    mut gate: ResMut<EscGate>,
) {
    let EscProbes {
        modals,
        combos,
        text,
        toast,
    } = probes;
    *gate = EscGate {
        overlay_open: !modals.is_empty()
            || report.is_some_and(|m| m.0)
            || menu.is_some_and(|m| m.0),
        popup_open: combos.iter().any(|c| c.is_open),
        text_focused: !text.is_empty(),
        toast_focused: !toast.is_empty(),
        viewpoint_open: viewpoint.is_some_and(|v| v.editing),
    };
}

/// Whether the screen on display has a step inside itself that *Back* takes first (the steps of
/// *Plan a drive*). The screen keeps it up to date; the shell then sends [`ScreenBack`] instead of
/// leaving the screen.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScreenBackSteps(pub bool);

/// *Back* for the screen itself: go one step back inside it (see [`ScreenBackSteps`]).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenBack;

/// Back in a shell screen: the screen's own step first, then one level up (the back stack), Home
/// when the stack is empty, nothing on Home.
pub fn shell_back_system(
    mut actions: MessageReader<UiAction>,
    gate: Res<EscGate>,
    steps: Res<ScreenBackSteps>,
    mut back: MessageWriter<NavigateBack>,
    mut inner: MessageWriter<ScreenBack>,
) {
    for action in actions.read() {
        if *action == UiAction::Back && !gate.closed() {
            if steps.0 {
                inner.write(ScreenBack);
            } else {
                back.write(NavigateBack);
            }
        }
    }
}

/// Registers the gate and the system. Active only in the shell's screens.
pub fn build(app: &mut App) {
    app.init_resource::<EscGate>()
        .init_resource::<ScreenBackSteps>()
        .add_message::<ScreenBack>()
        .add_systems(PreUpdate, compute_esc_gate)
        .add_systems(
            Update,
            shell_back_system
                .after(opts_gui::actions::ActionInput)
                .run_if(not(in_state(AppState::InGame)))
                .run_if(not(in_state(AppState::Loading))),
        );
}
