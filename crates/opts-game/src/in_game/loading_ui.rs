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

/// *Details* button of the outcome block.
#[derive(Component, Debug, Clone, Copy)]
pub struct DetailsBtn;
/// *Continue* button of *Ready with problems*.
#[derive(Component, Debug, Clone, Copy)]
pub struct ContinueBtn;
/// *Retry* button of *Failed*.
#[derive(Component, Debug, Clone, Copy)]
pub struct RetryBtn;
/// *Choose another* button of *Failed*.
#[derive(Component, Debug, Clone, Copy)]
pub struct ChooseAnotherBtn;
/// *Back* button of *Failed*.
#[derive(Component, Debug, Clone, Copy)]
pub struct LoadingBackBtn;
/// The stage sentence.
#[derive(Component, Debug, Clone, Copy)]
pub struct StageText;
/// The percentage text.
#[derive(Component, Debug, Clone, Copy)]
pub struct PercentText;
/// The container the outcome (result line, problems, buttons) is built into.
#[derive(Component, Debug, Clone, Copy)]
pub struct OutcomeBox;
/// A text of the manifest, by tile and line.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManifestText(pub Tile, pub u8);
/// The *Cancel loading?* dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct CancelDialog;
/// *Keep loading* in the dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct KeepLoadingBtn;
/// *Cancel loading* in the dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct ConfirmCancelBtn;

/// The three tiles of the manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    /// Map and start point.
    Map,
    /// Vehicle and livery.
    Vehicle,
    /// Date, time and weather.
    When,
}

mod spawn;
pub use spawn::spawn_loading_screen_ui;

/// The stage sentence for the state: its key and variables.
pub fn stage_message(state: &SimulationLoadState) -> (&'static str, L10nArgs) {
    match state.stage {
        LoadStage::InitEngine if state.waiting_for_catalogue => {
            ("load-stage-catalogue", L10nArgs::default())
        }
        LoadStage::InitEngine => ("load-stage-resolve", L10nArgs::default()),
        LoadStage::BuildTerrain => (
            "load-stage-map",
            L10nArgs(vec![
                ("done".into(), state.tiles_done.into()), // l10n-ok: variable name
                ("total".into(), state.tiles_total.into()), // l10n-ok: variable name
            ]),
        ),
        LoadStage::SetupAtmosphere => ("load-stage-light", L10nArgs::default()),
        LoadStage::SpawnVehicle => ("load-stage-vehicle", L10nArgs::default()),
        LoadStage::CalibrateControls => ("load-stage-start", L10nArgs::default()),
        LoadStage::Ready => ("load-stage-ready", L10nArgs::default()),
        LoadStage::Failed => ("load-stage-failed", L10nArgs::default()),
    }
}

/// The manifest texts of the session that is loading.
pub fn manifest_lines(
    loc: &Localization,
    weather: &opts_bevy::WeatherCatalogue,
    session: &InGameSessionInfo,
    setup: &NewGameSetup,
) -> [(Tile, [String; 2]); 3] {
    let when = NewGameSetup {
        hour: session.start_hour,
        minute: session.start_minute,
        weather_preset: session.weather_preset.clone(),
        ..setup.clone()
    };
    [
        (
            Tile::Map,
            [session.map_name.clone(), session.entry_point.clone()],
        ),
        (
            Tile::Vehicle,
            if session.vehicle_none {
                [loc.t("wiz-no-vehicle"), loc.t("load-no-vehicle-sub")]
            } else {
                [session.vehicle_name.clone(), session.livery_name.clone()]
            },
        ),
        (
            Tile::When,
            [
                crate::new_game_screen::texts::date_text(loc, &when),
                format!(
                    "{} · {}",
                    crate::new_game_screen::planner::clock_text(&when),
                    crate::new_game_screen::texts::weather_name(loc, weather, &when)
                ),
            ],
        ),
    ]
}

mod outcome;
use outcome::build_outcome;
pub use outcome::OutcomeSig;

/// Keeps bar, percentage, stage sentence, manifest and outcome current.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_loading_ui(
    state: Res<SimulationLoadState>,
    report: Res<LoadReport>,
    session: Res<InGameSessionInfo>,
    setup: Res<NewGameSetup>,
    weather: Res<opts_bevy::WeatherCatalogue>,
    loc: Res<Localization>,
    mut fill: Query<&mut Node, With<ProgressBarFill>>,
    mut percent: Query<&mut Text, (With<PercentText>, Without<ManifestText>)>,
    mut stage: Query<(&mut L10nKey, &mut L10nArgs), With<StageText>>,
    mut manifest: Query<(&ManifestText, &mut Text), Without<PercentText>>,
    outcome_box: Query<Entity, With<OutcomeBox>>,
    mut shown: Local<Option<OutcomeSig>>,
    mut commands: Commands,
) {
    let pct = (state.progress * 100.0).clamp(0.0, 100.0);
    for mut n in &mut fill {
        let w = Val::Percent(pct.max(2.0));
        if n.width != w {
            n.width = w;
        }
    }
    for mut t in &mut percent {
        let s = format!("{pct:.0}%");
        if t.0 != s {
            t.0 = s;
        }
    }
    let (key, args) = stage_message(&state);
    for (mut k, mut a) in &mut stage {
        if k.0 != key {
            k.0 = key;
        }
        if *a != args {
            *a = args.clone();
        }
    }
    let lines = manifest_lines(&loc, &weather, &session, &setup);
    for (m, mut t) in &mut manifest {
        let new = &lines
            .iter()
            .find(|(tile, _)| *tile == m.0)
            .map(|(_, l)| l[usize::from(m.1)].clone());
        if let Some(s) = new {
            if t.0 != *s {
                t.0.clone_from(s);
            }
        }
    }
    let sig = OutcomeSig {
        outcome: report.outcome,
        problems: report.blocking_count(),
        details: report.details_open,
    };
    if shown.as_ref() != Some(&sig) {
        if let Ok(b) = outcome_box.single() {
            commands.entity(b).despawn_related::<Children>();
            build_outcome(&mut commands, b, &report);
        }
        *shown = Some(sig);
    }
}

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
