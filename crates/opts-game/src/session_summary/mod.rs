//! The end-of-drive summary on Home (`docs/src/roadmap/phase-9/screens.md` section 5): duration,
//! distance, top speed, problems logged with a *Report* button, and *Drive again*, *Change
//! vehicle*, *Home*. Only measures that exist are shown.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::reporting::{request_report, ReportModalData, ReportModalState};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::drive_history::DriveRecord;
use crate::home::StartDrive;
use crate::new_game_screen::types::NewGameSetup;
use crate::pause::stats::{format_duration, SessionStats};
use crate::shell::frame::ShellBanners;
use crate::shell::{keys, nav::Navigate};

/// The drive that just ended.
#[derive(Resource, Debug, Clone)]
pub struct LastSession {
    /// The choices of the drive (for *Drive again*).
    pub record: DriveRecord,
    /// What was measured.
    pub stats: SessionStats,
}

impl LastSession {
    /// The summary of a drive started from `setup`.
    pub fn new(setup: &NewGameSetup, stats: SessionStats) -> Self {
        Self {
            record: DriveRecord::from_setup(setup, 0),
            stats,
        }
    }
}

/// The summary card.
#[derive(Component, Debug, Clone, Copy)]
pub struct SummaryCard;

/// Buttons of the card.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryAction {
    /// Same choices again.
    Again,
    /// Back to *Drive* with the map kept.
    ChangeVehicle,
    /// Dismiss.
    Home,
    /// Open the report dialog.
    Report,
}

/// The decimal separator of the language.
fn decimal(text: String, locale: &str) -> String {
    if locale.starts_with("de") {
        text.replace('.', ",")
    } else {
        text
    }
}

fn line(key: &'static str, args: L10nArgs) -> impl Bundle {
    (
        Text::new(key),
        L10nKey(key),
        args,
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurface),
    )
}

/// Duration, distance and top speed.
fn stat_row(c: &mut ChildSpawnerCommands, stats: &SessionStats, locale: &str) {
    c.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(24.0),
        row_gap: Val::Px(4.0),
        ..default()
    })
    .with_children(|row| {
        row.spawn(line(
            keys::SUMMARY_DURATION,
            L10nArgs::one("time", format_duration(stats.duration_s)),
        ));
        if stats.distance_m < 1000.0 {
            row.spawn(line(
                keys::SUMMARY_DISTANCE_M,
                L10nArgs::one("m", format!("{:.0}", stats.distance_m)),
            ));
        } else {
            row.spawn(line(
                keys::SUMMARY_DISTANCE,
                L10nArgs::one(
                    "km",
                    decimal(format!("{:.1}", stats.distance_m / 1000.0), locale),
                ),
            ));
        }
        row.spawn(line(
            keys::SUMMARY_MAX_SPEED,
            L10nArgs::one("kmh", format!("{:.0}", stats.max_speed_kmh())),
        ));
    });
}

/// The number of problems with the button that opens the report, when there are any.
fn problems_row(c: &mut ChildSpawnerCommands, stats: &SessionStats) {
    let problems = stats.problems();
    if problems > 0 {
        c.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn(line(
                keys::SUMMARY_PROBLEMS,
                L10nArgs::one("count", problems),
            ));
            let report = ButtonBuilder::tonal(keys::SUMMARY_REPORT)
                .with_key(keys::SUMMARY_REPORT)
                .spawn_child(row);
            row.commands()
                .entity(report)
                .insert((SummaryAction::Report, UiId::new("summary.report")));
        });
    }
}

/// *Again*, *Change vehicle* and *Home*.
fn action_row(c: &mut ChildSpawnerCommands) {
    c.spawn(Node {
        column_gap: Val::Px(12.0),
        flex_wrap: FlexWrap::Wrap,
        row_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|row| {
        for (action, key, id, primary) in [
            (SummaryAction::Again, keys::SUMMARY_AGAIN, "again", true),
            (
                SummaryAction::ChangeVehicle,
                keys::SUMMARY_CHANGE_VEHICLE,
                "change-vehicle",
                false,
            ),
            (SummaryAction::Home, keys::SUMMARY_HOME, "home", false),
        ] {
            let builder = if primary {
                ButtonBuilder::filled(key)
            } else {
                ButtonBuilder::tonal(key)
            };
            let button = builder.with_key(key).spawn_child(row);
            row.commands()
                .entity(button)
                .insert((action, UiId::new(format!("summary.{id}"))));
        }
    });
}

/// Builds the card in the banner area when Home opens after a drive that moved.
pub fn spawn_summary(
    mut commands: Commands,
    last: Option<Res<LastSession>>,
    area: Query<Entity, With<ShellBanners>>,
    existing: Query<(), With<SummaryCard>>,
    loc: Res<Localization>,
) {
    let (Some(last), Ok(area)) = (last, area.single()) else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    let stats = &last.stats;
    let card = commands
        .spawn((
            Node {
                margin: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::Large),
            SummaryCard,
            Name::new("SessionSummary"),
        ))
        .id();
    commands.entity(area).add_child(card);
    let locale = loc.locale().to_owned();
    commands.entity(card).with_children(|c| {
        c.spawn((
            Text::new(keys::SUMMARY_TITLE),
            L10nKey(keys::SUMMARY_TITLE),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        stat_row(c, stats, &locale);
        problems_row(c, stats);
        action_row(c);
    });
}

/// Removes the card when Home is left.
pub fn clear_summary(mut commands: Commands, cards: Query<Entity, With<SummaryCard>>) {
    for card in &cards {
        commands.entity(card).despawn();
    }
}

/// The report dialog's data and state, with the clock its request reads.
#[derive(SystemParam)]
pub struct SummaryReport<'w> {
    time: Res<'w, Time<Virtual>>,
    report: ResMut<'w, ReportModalData>,
    state: Res<'w, State<ReportModalState>>,
    next: ResMut<'w, NextState<ReportModalState>>,
}

/// Where the card's buttons send the player.
#[derive(SystemParam)]
pub struct SummaryOut<'w> {
    start: MessageWriter<'w, StartDrive>,
    navigate: MessageWriter<'w, Navigate>,
}

/// The buttons of the card.
pub fn summary_actions(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    actions: Query<&SummaryAction>,
    cards: Query<Entity, With<SummaryCard>>,
    last: Option<Res<LastSession>>,
    dialog: SummaryReport,
    out: SummaryOut,
) {
    let SummaryReport {
        time,
        mut report,
        state: report_state,
        next: mut report_next,
    } = dialog;
    let SummaryOut {
        mut start,
        mut navigate,
    } = out;
    for Activated(entity) in activated.read() {
        let Ok(action) = actions.get(*entity) else {
            continue;
        };
        match action {
            SummaryAction::Report => {
                request_report(&time, &mut report, &report_state, &mut report_next);
                continue;
            }
            SummaryAction::Again => {
                if let Some(last) = &last {
                    start.write(StartDrive(last.record.clone()));
                }
            }
            SummaryAction::ChangeVehicle => {
                navigate.write(Navigate::push("drive"));
            }
            SummaryAction::Home => {}
        }
        for card in &cards {
            commands.entity(card).despawn();
        }
        commands.remove_resource::<LastSession>();
    }
}

/// Registers the summary.
pub struct SessionSummaryPlugin;

impl Plugin for SessionSummaryPlugin {
    fn build(&self, app: &mut App) {
        crate::pause::ensure_report_state(app);
        app.add_systems(
            Update,
            (spawn_summary, summary_actions).run_if(in_state(AppState::MainMenu)),
        )
        .add_systems(OnExit(AppState::MainMenu), clear_summary);
    }
}

#[cfg(test)]
mod tests;
