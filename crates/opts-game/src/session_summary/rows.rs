//! The summary's rows: the duration, distance and top speed, the problems with the report
//! button, and the action buttons.

use bevy::prelude::*;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::prelude::*;

use crate::pause::stats::{format_duration, SessionStats};
use crate::shell::keys;

use super::SummaryAction;

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
pub(super) fn stat_row(c: &mut ChildSpawnerCommands, stats: &SessionStats, locale: &str) {
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
pub(super) fn problems_row(c: &mut ChildSpawnerCommands, stats: &SessionStats) {
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
pub(super) fn action_row(c: &mut ChildSpawnerCommands) {
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
