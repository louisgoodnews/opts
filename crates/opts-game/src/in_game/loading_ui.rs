//! The loading screen: themed, plain-language stages with real counts, the manifest of the
//! session that is really loading, a controls card, and the three outcomes *Ready* (continues by
//! itself), *Ready with N problems* and *Failed*. Specification:
//! `docs/src/roadmap/phase-9/wizard-and-loading.md` section 7.

use bevy::prelude::*;
use opts_gui::actions::UiAction;
use opts_gui::components::modal::{ModalEvent, ModalRoot};
use opts_gui::interaction::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::patterns::glyphs;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::in_game::load_report::{
    AcceptLoadProblems, LoadOutcome, LoadProblem, LoadReport, Severity,
};
use crate::in_game::types::{
    CancelLoadingBtn, InGameEntity, InGameSessionInfo, LoadStage, LoadingScreenRoot,
    ProgressBarFill, SimulationLoadState,
};
use crate::new_game_screen::types::NewGameSetup;
use crate::new_game_screen::ui::{column, glyph, keyed, keyed_args, plain, row};
use crate::shell::{spawn_screen_root, ShellContent};

mod components;
mod texts;
mod update;
pub use components::{
    CancelDialog, ChooseAnotherBtn, ConfirmCancelBtn, ContinueBtn, DetailsBtn, KeepLoadingBtn,
    LoadingBackBtn, ManifestText, OutcomeBox, PercentText, RetryBtn, StageText, Tile,
};
pub use texts::{manifest_lines, stage_message};
pub use update::{update_loading_ui, LoadingData, LoadingNodes};

mod outcome;
use outcome::build_outcome;
pub use outcome::OutcomeSig;

mod spawn;
pub use spawn::spawn_loading_screen_ui;

mod dialogs;
pub use dialogs::{
    announce_minor_problems, ask_to_cancel, handle_cancel, handle_minor_dialog, MinorCloseBtn,
    MinorDialog, MinorToast,
};

/// The developer overlay (F10) never draws over the loading screen.
pub fn hide_overlay_while_loading(
    screen: Query<(), With<LoadingScreenRoot>>,
    mut overlay: Query<&mut Visibility, With<crate::in_game::omsi_world::DebugOverlayText>>,
) {
    if screen.is_empty() {
        return;
    }
    for mut v in &mut overlay {
        *v = Visibility::Hidden;
    }
}

/// Registers the systems of the loading screen.
pub fn build(app: &mut App) {
    app.init_resource::<LoadReport>()
        .init_resource::<AcceptLoadProblems>()
        .init_resource::<crate::in_game::load_report::AcceptLoadProblemsAlways>()
        .add_systems(
            Update,
            (
                announce_minor_problems,
                handle_minor_dialog,
                // Before the pipeline: when it ends the screen in the same frame (accepted
                // problems), the outcome box is rebuilt first and not on a despawned entity.
                update_loading_ui.before(crate::in_game::loading_pipeline::update_loading_progress),
                handle_cancel,
                hide_overlay_while_loading.after(crate::in_game::omsi_world::update_debug_overlay),
            )
                .run_if(in_state(AppState::InGame)),
        );
}
