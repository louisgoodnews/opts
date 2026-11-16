//! The time and date blocks of the *When* and *Where* steps: the time steppers with their source
//! hints, and the date steppers with the era hint.

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;
use opts_gui::prelude::*;

use super::{stepper, EraHint, TimeSourceLabel};
use crate::new_game_screen::planner::{era_hint, month_key, Field, TimeSource};
use crate::new_game_screen::ui::{column, keyed, keyed_args};
use crate::new_game_screen::BuildCtx;
use crate::shell::calendar;

pub(super) fn time_block(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let s = ctx.setup;
    let block = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(560.0),
                ..column(12.0)
            },
            Name::new("TimeBlock"),
        ))
        .id();
    let hour = stepper(
        commands,
        Field::Hour,
        format!("{:02}", s.hour),
        "wizard.when",
    );
    let minute = stepper(
        commands,
        Field::Minute,
        format!("{:02}", s.minute),
        "wizard.when",
    );
    let time = calendar::spawn_stepper_row(commands, &[hour, minute]);
    commands.entity(block).add_child(time);
    for (source, key) in [
        (TimeSource::Map, "wiz-when-source-map"),
        (TimeSource::Now, "wiz-when-source-now"),
    ] {
        let label = commands
            .spawn((
                keyed(key, TypeScale::Label, ColorRole::Info),
                TimeSourceLabel(source),
                Node {
                    display: Display::None,
                    ..default()
                },
            ))
            .id();
        commands.entity(block).add_child(label);
    }
    block
}

/// The date steppers and the era hint (shown only outside the map's years); the map step shows
/// them, the one place where the date is chosen.
pub fn date_block(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let s = ctx.setup;
    let block = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(560.0),
                ..column(8.0)
            },
            Name::new("DateBlock"),
        ))
        .id();
    let label = commands
        .spawn(keyed(
            "wiz-where-date",
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let day = stepper(commands, Field::Day, s.day.to_string(), "wizard.where");
    let month = stepper(
        commands,
        Field::Month,
        ctx.loc.t(month_key(s.month)),
        "wizard.where",
    );
    let year = stepper(commands, Field::Year, s.year.to_string(), "wizard.where");
    let date = calendar::spawn_stepper_row(commands, &[day, month, year]);
    let hint = ctx
        .catalogue
        .map_by_name(&s.selected_map)
        .and_then(|m| era_hint(&m.years, s.year));
    let hint_node = commands
        .spawn((
            keyed_args(
                "wiz-when-era-hint",
                L10nArgs(vec![
                    ("from".into(), hint.map_or(0, |h| h.0).into()),
                    ("to".into(), hint.map_or(0, |h| h.1).into()),
                ]),
                TypeScale::Label,
                ColorRole::Info,
            ),
            EraHint,
            Node {
                display: if hint.is_some() {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
        ))
        .id();
    commands
        .entity(block)
        .add_children(&[label, date, hint_node]);
    block
}
