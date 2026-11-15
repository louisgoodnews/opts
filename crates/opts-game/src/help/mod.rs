//! Help: controls, guides, report an issue and About (`docs/src/roadmap/phase-9/screens.md` §6).
//!
//! F1 opens it (D-UI-06): the page in the shell, an overlay during a drive. *Report an issue* is
//! reachable from here and from the pause menu; the control-protocol command `report` is
//! unchanged.

mod about;
mod controls;
mod overlay;
mod page;
mod params;
pub mod scroll;
mod tabs;
pub mod vehicle_controls;

use crate::app_state::AppState;
use crate::pause::Pause;
use crate::shell::keys;
use crate::shell::nav::Navigate;
use bevy::prelude::*;

pub use overlay::{HelpOverlay, open_overlay};
pub use params::{F1Gate, ReportRequest, VehicleProbe};

use page::{despawn_help, spawn_help};
use tabs::{report_click, sync_pane, sync_tabs, tab_clicks};

/// Root of the Help page.
#[derive(Component, Debug, Clone, Copy)]
pub struct HelpRoot;

/// The right-hand pane of the Help page.
#[derive(Component, Debug, Clone, Copy)]
pub struct HelpPane;

/// The tabs of the Help page.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpTab {
    /// Key table.
    #[default]
    Controls,
    /// Guides (not available yet).
    Guides,
    /// Report an issue.
    Report,
    /// Version and licences.
    About,
}

impl HelpTab {
    pub(super) const ALL: [Self; 4] = [Self::Controls, Self::Guides, Self::Report, Self::About];

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Controls => keys::HELP_TAB_CONTROLS,
            Self::Guides => keys::HELP_TAB_GUIDES,
            Self::Report => keys::HELP_TAB_REPORT,
            Self::About => keys::HELP_TAB_ABOUT,
        }
    }

    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Controls => "controls",
            Self::Guides => "guides",
            Self::Report => "report",
            Self::About => "about",
        }
    }

    pub(super) fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.id() == text)
    }
}

/// The open tab.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HelpState {
    /// Which tab.
    pub tab: HelpTab,
}

/// The *Report an issue* button of the page.
#[derive(Component, Debug, Clone, Copy)]
pub struct ReportButton;

/// Help plugin.
pub struct HelpPlugin;

impl Plugin for HelpPlugin {
    fn build(&self, app: &mut App) {
        crate::pause::ensure_report_state(app);
        app.init_resource::<HelpState>()
            .init_resource::<Pause>()
            .add_systems(
                Update,
                (
                    controls::sync_action_chips,
                    vehicle_controls::fill_vehicle_controls,
                ),
            )
            .add_systems(OnEnter(AppState::Help), spawn_help)
            .add_systems(OnExit(AppState::Help), despawn_help)
            .add_systems(
                Update,
                (tab_clicks, sync_tabs, sync_pane, report_click).run_if(in_state(AppState::Help)),
            )
            .add_systems(
                Update,
                (f1_opens_help, overlay::overlay_actions, scroll::scroll_help)
                    .run_if(not(in_state(AppState::Loading))),
            );
    }
}

/// F1 opens Help: the page in the shell, the overlay (over a paused scene) in a drive.
pub fn f1_opens_help(
    input: F1Gate,
    state: Res<State<AppState>>,
    mut pause: ResMut<Pause>,
    mut time: ResMut<Time<Virtual>>,
    mut navigate: MessageWriter<Navigate>,
    mut commands: Commands,
) {
    let F1Gate {
        keys_in,
        action_keys,
        gate,
        overlays,
    } = input;
    if !opts_bevy::action_keys_of(&action_keys).just_pressed(&keys_in, "help")
        || gate.closed()
        || !overlays.is_empty()
    {
        return;
    }
    match state.get() {
        AppState::InGame => {
            if !pause.open {
                pause.resume_time = !time.is_paused();
                time.pause();
                pause.open = true;
            } else if let Some(root) = pause.root.take() {
                commands.entity(root).try_despawn();
            }
            open_overlay(&mut commands);
        }
        AppState::Help => {}
        _ => {
            navigate.write(Navigate::push("help"));
        }
    }
}

#[cfg(test)]
mod tests;
