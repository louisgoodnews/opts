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
use crate::in_game::load_report::{LoadOutcome, LoadProblem, LoadReport, Severity};
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

fn tile_label(t: Tile) -> (&'static str, ColorRole) {
    match t {
        Tile::Map => ("load-tile-map", ColorRole::Primary),
        Tile::Vehicle => ("load-tile-vehicle", ColorRole::Tertiary),
        Tile::When => ("load-tile-when", ColorRole::Secondary),
    }
}

fn manifest_tile(commands: &mut Commands, tile: Tile) -> Entity {
    let (label, role) = tile_label(tile);
    let frame = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..column(4.0)
            },
            BackgroundRole(ColorRole::SurfaceContainerLow),
            BorderRole(ColorRole::OutlineVariant),
            RadiusToken(RadiusLevel::Medium),
            Name::new("ManifestTile"),
        ))
        .id();
    let head = commands.spawn(keyed(label, TypeScale::Label, role)).id();
    let title = commands
        .spawn((
            plain(String::new(), TypeScale::TitleMedium, ColorRole::OnSurface),
            ManifestText(tile, 0),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
        ))
        .id();
    let sub = commands
        .spawn((
            plain(
                String::new(),
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ),
            ManifestText(tile, 1),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
        ))
        .id();
    commands.entity(frame).add_children(&[head, title, sub]);
    frame
}

fn progress_block(commands: &mut Commands) -> Entity {
    let block = commands.spawn(column(8.0)).id();
    let line = commands
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            ..row(12.0)
        })
        .id();
    let stage = commands
        .spawn((
            keyed(
                "load-stage-resolve",
                TypeScale::BodyLarge,
                ColorRole::OnSurface,
            ),
            StageText,
        ))
        .id();
    let percent = commands
        .spawn((
            plain("0%", TypeScale::BodyLarge, ColorRole::Primary),
            PercentText,
        ))
        .id();
    commands.entity(line).add_children(&[stage, percent]);
    let track = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(12.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHighest),
            RadiusToken(RadiusLevel::Full),
        ))
        .id();
    let fill = commands
        .spawn((
            Node {
                width: Val::Percent(5.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundRole(ColorRole::Primary),
            RadiusToken(RadiusLevel::Full),
            ProgressBarFill,
        ))
        .id();
    commands.entity(track).add_child(fill);
    commands.entity(block).add_children(&[line, track]);
    block
}

/// The keys a driver needs while waiting (the bindings that exist today).
const CONTROLS: [(&str, &str); 8] = [
    ("W / ↑", "load-controls-accelerate"),
    ("S / ↓", "load-controls-brake"),
    ("← →", "load-controls-steer"),
    (".", "load-controls-parking"),
    ("D  N  R", "load-controls-gears"),
    ("1 – 5", "load-controls-cameras"),
    ("P", "load-controls-pause"),
    ("Esc", "load-controls-menu"),
];

fn controls_card(commands: &mut Commands) -> Entity {
    let card = commands
        .spawn((
            Node {
                padding: UiRect::all(Val::Px(12.0)),
                ..column(8.0)
            },
            BackgroundRole(ColorRole::SurfaceContainerLow),
            RadiusToken(RadiusLevel::Medium),
            Name::new("ControlsCard"),
        ))
        .id();
    let title = commands
        .spawn(keyed(
            "load-controls-title",
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let grid = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(24.0),
            row_gap: Val::Px(6.0),
            ..row(24.0)
        })
        .id();
    for (keys, action) in CONTROLS {
        let item = commands.spawn(row(8.0)).id();
        let chip = commands
            .spawn((
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundRole(ColorRole::SurfaceContainerHighest),
                RadiusToken(RadiusLevel::Small),
            ))
            .id();
        let k = commands
            .spawn(plain(keys, TypeScale::Label, ColorRole::OnSurface))
            .id();
        commands.entity(chip).add_child(k);
        let a = commands
            .spawn(keyed(
                action,
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id();
        commands.entity(item).add_children(&[chip, a]);
        commands.entity(grid).add_child(item);
    }
    commands.entity(card).add_children(&[title, grid]);
    card
}

/// Spawns the loading screen (inside the shell's content area when there is one).
pub fn spawn_loading_screen_ui(commands: &mut Commands, shell: &ShellContent) -> Entity {
    let root = spawn_screen_root(commands, shell, "Loading");
    commands.entity(root).insert((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..column(0.0)
        },
        BackgroundRole(ColorRole::Surface),
        GlobalZIndex(100),
        InGameEntity,
        LoadingScreenRoot,
    ));
    let inner = commands
        .spawn(Node {
            width: Val::Percent(92.0),
            max_width: Val::Px(960.0),
            ..column(16.0)
        })
        .id();
    let title = commands
        .spawn(keyed(
            "load-title",
            TypeScale::Headline,
            ColorRole::OnSurface,
        ))
        .id();
    let manifest = commands.spawn(row(12.0)).id();
    let tiles: Vec<Entity> = [Tile::Map, Tile::Vehicle, Tile::When]
        .into_iter()
        .map(|t| manifest_tile(commands, t))
        .collect();
    commands.entity(manifest).add_children(&tiles);
    let progress = progress_block(commands);
    let outcome = commands
        .spawn((column(8.0), OutcomeBox, Name::new("Outcome")))
        .id();
    let controls = controls_card(commands);
    let footer = commands.spawn(row(12.0)).id();
    let cancel = ButtonBuilder::outlined("load-cancel")
        .with_key("load-cancel")
        .spawn(commands);
    commands
        .entity(cancel)
        .insert((CancelLoadingBtn, UiId::new("loading.cancel")));
    commands.entity(footer).add_child(cancel);
    commands
        .entity(inner)
        .add_children(&[title, manifest, progress, outcome, controls, footer]);
    commands.entity(root).add_child(inner);
    root
}

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
                    crate::new_game_screen::texts::weather_name(loc, &when)
                ),
            ],
        ),
    ]
}

/// What the outcome block shows right now (rebuilt only when this changes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeSig {
    outcome: LoadOutcome,
    problems: usize,
    details: bool,
}

/// Keeps bar, percentage, stage sentence, manifest and outcome current.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_loading_ui(
    state: Res<SimulationLoadState>,
    report: Res<LoadReport>,
    session: Res<InGameSessionInfo>,
    setup: Res<NewGameSetup>,
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
    let lines = manifest_lines(&loc, &session, &setup);
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

fn problem_row(commands: &mut Commands, p: &LoadProblem, details: bool) -> Entity {
    let wrap = commands.spawn(column(2.0)).id();
    let icon = if p.fatal {
        glyphs::ERROR
    } else {
        glyphs::WARNING
    };
    let role = if p.fatal {
        ColorRole::Error
    } else {
        ColorRole::Warning
    };
    let head = commands.spawn(row(8.0)).id();
    let g = commands.spawn(glyph(icon, 20.0, role)).id();
    let args = L10nArgs(
        p.args
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().into()))
            .collect(),
    );
    let what = commands
        .spawn(keyed_args(
            p.key,
            args,
            TypeScale::BodyMedium,
            ColorRole::OnSurface,
        ))
        .id();
    commands.entity(head).add_children(&[g, what]);
    let mut kids = vec![head];
    if let Some(cause) = p.cause_key {
        kids.push(
            commands
                .spawn((
                    keyed(cause, TypeScale::Label, ColorRole::OnSurfaceVariant),
                    Node {
                        margin: UiRect::left(Val::Px(28.0)),
                        ..default()
                    },
                ))
                .id(),
        );
    }
    if details && !p.detail.is_empty() {
        kids.push(
            commands
                .spawn((
                    plain(
                        p.detail.clone(),
                        TypeScale::Label,
                        ColorRole::OnSurfaceVariant,
                    ),
                    Node {
                        margin: UiRect::left(Val::Px(28.0)),
                        ..default()
                    },
                ))
                .id(),
        );
    }
    commands.entity(wrap).add_children(&kids);
    wrap
}

fn outcome_button(
    commands: &mut Commands,
    key: &'static str,
    primary: bool,
    id: &'static str,
) -> Entity {
    let b = if primary {
        ButtonBuilder::filled(key)
    } else {
        ButtonBuilder::tonal(key)
    }
    .with_key(key)
    .spawn(commands);
    commands.entity(b).insert(UiId::new(id));
    b
}

fn build_outcome(commands: &mut Commands, container: Entity, report: &LoadReport) {
    let mut kids = Vec::new();
    match report.outcome {
        LoadOutcome::Loading | LoadOutcome::Ready => {}
        LoadOutcome::ReadyWithProblems | LoadOutcome::Failed => {
            let failed = report.outcome == LoadOutcome::Failed;
            let (key, role) = if failed {
                ("load-failed", ColorRole::Error)
            } else {
                ("load-ready-problems", ColorRole::Warning)
            };
            kids.push(
                commands
                    .spawn(keyed_args(
                        key,
                        L10nArgs::one("count", report.blocking_count()), // l10n-ok: variable name
                        TypeScale::TitleLarge,
                        role,
                    ))
                    .id(),
            );
            for p in report
                .problems
                .iter()
                .filter(|p| p.severity() == Severity::Blocking)
            {
                kids.push(problem_row(commands, p, report.details_open || failed));
            }
            kids.push(outcome_buttons(commands, failed));
        }
    }
    commands.entity(container).add_children(&kids);
}

fn outcome_buttons(commands: &mut Commands, failed: bool) -> Entity {
    let line = commands.spawn(row(12.0)).id();
    let mut buttons = Vec::new();
    if failed {
        let retry = outcome_button(commands, "load-retry", true, "loading.retry");
        commands.entity(retry).insert((RetryBtn, DefaultFocus));
        let another = outcome_button(
            commands,
            "load-choose-another",
            false,
            "loading.choose-another",
        );
        commands.entity(another).insert(ChooseAnotherBtn);
        let back = outcome_button(commands, "load-back", false, "loading.back");
        commands.entity(back).insert(LoadingBackBtn);
        buttons.extend([retry, another, back]);
    } else {
        let details = outcome_button(commands, "load-details", false, "loading.details");
        commands.entity(details).insert(DetailsBtn);
        let cont = outcome_button(commands, "load-continue", true, "loading.continue");
        commands.entity(cont).insert((ContinueBtn, DefaultFocus));
        buttons.extend([details, cont]);
    }
    commands.entity(line).add_children(&buttons);
    line
}

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

/// The cancel button, the dialog's buttons and `Esc`.
#[allow(clippy::too_many_arguments)]
pub fn handle_cancel(
    mut activated: MessageReader<Activated>,
    mut actions: MessageReader<UiAction>,
    mut modal_events: MessageReader<ModalEvent>,
    cancel: Query<(), With<CancelLoadingBtn>>,
    keep: Query<(), With<KeepLoadingBtn>>,
    confirm: Query<(), With<ConfirmCancelBtn>>,
    dialogs: Query<Entity, With<CancelDialog>>,
    other_modals: Query<(), (With<ModalRoot>, Without<CancelDialog>)>,
    screen: Query<(), With<LoadingScreenRoot>>,
    report: Res<LoadReport>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
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

/// The toast's *Details* opens the list; *Close* and `Esc` close it.
#[allow(clippy::too_many_arguments)]
pub fn handle_minor_dialog(
    mut activated: MessageReader<Activated>,
    mut modal_events: MessageReader<ModalEvent>,
    actions: Query<&opts_gui::components::toast::ToastActionButton>,
    close: Query<(), With<MinorCloseBtn>>,
    dialogs: Query<Entity, With<MinorDialog>>,
    toast: Option<Res<MinorToast>>,
    report: Res<LoadReport>,
    mut commands: Commands,
) {
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

/// Registers the systems of the loading screen.
pub fn build(app: &mut App) {
    app.init_resource::<LoadReport>().add_systems(
        Update,
        (
            announce_minor_problems,
            handle_minor_dialog,
            update_loading_ui,
            handle_cancel,
            hide_overlay_while_loading.after(crate::in_game::omsi_world::update_debug_overlay),
        )
            .run_if(in_state(AppState::InGame)),
    );
}
