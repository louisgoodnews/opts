//! F1 reporting: capture the window, show a modal with a text area, save PNG + `.report`.
//!
//! Flow (`docs/src/architecture/reporting-feature.md`): F1 → [`ReportModalState::Capturing`]
//! (pause virtual time, gate input, request screenshot) → [`ReportModalState::Open`] (modal) →
//! Report / Cancel / Escape → [`ReportModalState::Inactive`] (resume). The application fills
//! [`ReportContext`] with session facts; systems that read input should run with
//! [`not_modal_active`].
//!
//! Requires `opts_gui::OptsGuiPlugin` and Bevy's default plugins (window, render, UI, states).

pub mod capture;
pub mod format;
pub mod modal;
pub mod pause;
pub mod save;
pub mod state;
pub mod trigger;

use bevy::prelude::*;

pub use format::{format_report, ReportMeta};
pub use modal::ReportsDir;
pub use save::{ensure_reports_dir, reports_dir, save_report, SavedReport};
pub use state::{not_modal_active, ModalActive, ReportContext, ReportModalData, ReportModalState};
pub use trigger::REPORT_KEY;

/// Registers the reporting feature.
#[derive(Default)]
pub struct ReportingPlugin;

impl Plugin for ReportingPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ReportModalState>()
            .init_resource::<ReportModalData>()
            .init_resource::<ModalActive>()
            .init_resource::<ReportContext>()
            .init_resource::<capture::CaptureChannel>()
            .init_resource::<capture::EncodeTask>()
            .init_resource::<modal::SaveTask>()
            .init_resource::<modal::ReportsDir>()
            .add_systems(Startup, init_reports_dir)
            .add_systems(Update, trigger::trigger_report_system)
            .add_systems(
                Update,
                capture::poll_capture.run_if(in_state(ReportModalState::Capturing)),
            )
            .add_systems(
                Update,
                (
                    modal::sync_text,
                    modal::handle_modal_actions,
                    modal::poll_save,
                )
                    .chain()
                    .run_if(in_state(ReportModalState::Open)),
            )
            .add_systems(
                OnEnter(ReportModalState::Capturing),
                (pause::on_report_start, capture::request_screenshot).chain(),
            )
            .add_systems(OnEnter(ReportModalState::Open), modal::spawn_modal)
            .add_systems(
                OnEnter(ReportModalState::Inactive),
                (pause::on_report_end, modal::despawn_modal),
            );
    }
}

/// Environment variable overriding the reports directory (tests, portable installs).
pub const REPORTS_DIR_ENV: &str = "OPTS_REPORTS_DIR";

fn init_reports_dir(mut dir: ResMut<ReportsDir>) {
    if let Some(custom) = std::env::var_os(REPORTS_DIR_ENV).map(std::path::PathBuf::from) {
        match std::fs::create_dir_all(&custom) {
            Ok(()) => {
                tracing::info!(
                    "reports directory: {} (from {REPORTS_DIR_ENV})",
                    custom.display()
                );
                dir.0 = Some(custom);
            }
            Err(e) => tracing::warn!("cannot create {}: {e}", custom.display()),
        }
        return;
    }
    match ensure_reports_dir() {
        Ok(path) => {
            tracing::info!("reports directory: {}", path.display());
            dir.0 = Some(path);
        }
        Err(e) => {
            tracing::warn!("cannot create the reports directory, reports cannot be saved: {e}")
        }
    }
}
