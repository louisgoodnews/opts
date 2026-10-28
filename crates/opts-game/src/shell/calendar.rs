//! Date, time and weather controls shared by the wizard's map step, its *When* step and the pause
//! menu's *Environment* page: the stepper arithmetic, fixed-size `[-] value [+]` steppers and the
//! frame of a choice chip.
//!
//! The arithmetic works on [`CalendarDate`] and an hour/minute pair, so the wizard
//! (`NewGameSetup`) and the running session (`SessionEnvironment`) step the same way. Labels use
//! the wizard's message keys (`wiz-when-*`, `wiz-month-*`).

use bevy::prelude::*;
use opts_bevy::environment::days_in_month;
use opts_bevy::CalendarDate;
use opts_gui::icons::icon_text;
use opts_gui::l10n::L10nKey;
use opts_gui::patterns::glyphs;
use opts_gui::prelude::*;
use opts_gui::ui_tree::A11yHidden;

/// Supported years.
pub const YEARS: std::ops::RangeInclusive<i32> = 1900..=2100;

/// A part of the date or time the steppers change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    /// Day of the month.
    Day,
    /// Month.
    Month,
    /// Year.
    Year,
    /// Hour.
    Hour,
    /// Minute.
    Minute,
}

impl Field {
    /// The date fields in display order.
    pub const DATE: [Self; 3] = [Self::Day, Self::Month, Self::Year];
    /// The time fields in display order.
    pub const TIME: [Self; 2] = [Self::Hour, Self::Minute];

    /// Stable id fragment (`day`, `month`, ...), used in `UiId`s.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Month => "month",
            Self::Year => "year",
            Self::Hour => "hour",
            Self::Minute => "minute",
        }
    }

    /// Message key of the stepper label.
    pub const fn label_key(self) -> &'static str {
        match self {
            Self::Day => "wiz-when-day",
            Self::Month => "wiz-when-month",
            Self::Year => "wiz-when-year",
            Self::Hour => "wiz-when-hour",
            Self::Minute => "wiz-when-minute",
        }
    }

    /// Width of the value box.
    pub const fn value_width(self) -> f32 {
        match self {
            Self::Day | Self::Hour | Self::Minute => 48.0,
            Self::Month => 96.0,
            Self::Year => 64.0,
        }
    }
}

/// Changes a date field by `delta` steps: days and months wrap, years clamp to [`YEARS`], and
/// the day is kept inside the month (31 January + 1 month = 28/29 February). Time fields are
/// ignored.
pub fn step_date(date: &mut CalendarDate, field: Field, delta: i32) {
    match field {
        Field::Day => {
            let n = days_in_month(date.year, date.month).max(1) as i32;
            date.day = ((date.day as i32 - 1 + delta).rem_euclid(n) + 1) as u32;
        }
        Field::Month => {
            date.month = ((date.month as i32 - 1 + delta).rem_euclid(12) + 1) as u32;
        }
        Field::Year => date.year = (date.year + delta).clamp(*YEARS.start(), *YEARS.end()),
        Field::Hour | Field::Minute => {}
    }
    *date = clamp_date(*date);
}

/// Changes a time field by `delta` steps: hours wrap, minutes move by 5 on the 5-minute grid
/// (by 1 when `fine`) and wrap. Date fields are ignored.
pub fn step_time(hour: &mut u32, minute: &mut u32, field: Field, delta: i32, fine: bool) {
    match field {
        Field::Hour => *hour = (*hour as i32 + delta).rem_euclid(24) as u32,
        Field::Minute => {
            let step = if fine { 1 } else { 5 };
            let m = *minute as i32 + delta * step;
            *minute = if fine {
                m.rem_euclid(60)
            } else {
                (m.div_euclid(5) * 5).rem_euclid(60)
            } as u32;
        }
        Field::Day | Field::Month | Field::Year => {}
    }
    *hour = (*hour).min(23);
    *minute = (*minute).min(59);
}

/// A valid date inside [`YEARS`].
pub fn clamp_date(date: CalendarDate) -> CalendarDate {
    CalendarDate::clamped(
        date.year.clamp(*YEARS.start(), *YEARS.end()),
        date.month,
        date.day,
    )
}

/// Message key of a month name.
pub fn month_key(month: u32) -> &'static str {
    const KEYS: [&str; 12] = [
        "wiz-month-1",
        "wiz-month-2",
        "wiz-month-3",
        "wiz-month-4",
        "wiz-month-5",
        "wiz-month-6",
        "wiz-month-7",
        "wiz-month-8",
        "wiz-month-9",
        "wiz-month-10",
        "wiz-month-11",
        "wiz-month-12",
    ];
    KEYS[(month.clamp(1, 12) - 1) as usize]
}

/// The value text of a date field (`month_name` turns a month into its localised name).
pub fn date_value(date: CalendarDate, field: Field, month_name: impl Fn(u32) -> String) -> String {
    match field {
        Field::Day => date.day.to_string(),
        Field::Month => month_name(date.month),
        Field::Year => date.year.to_string(),
        Field::Hour | Field::Minute => String::new(),
    }
}

/// The value text of a time field.
pub fn time_value(hour: u32, minute: u32, field: Field) -> String {
    match field {
        Field::Hour => format!("{hour:02}"),
        Field::Minute => format!("{minute:02}"),
        _ => String::new(),
    }
}

fn glyph(code: &'static str, size: f32, role: ColorRole) -> impl Bundle {
    (
        icon_text(code, size, TextColor::default().0),
        ForegroundRole(role),
        A11yHidden,
        Pickable::IGNORE,
    )
}

fn step_button(commands: &mut Commands, id: String, delta: i32, action: impl Bundle) -> Entity {
    let button = ButtonBuilder::text(String::new()).spawn(commands);
    commands.entity(button).insert((
        action,
        UiId::new(id),
        Tooltip::new(""),
        opts_gui::l10n::L10nTooltip(if delta < 0 {
            "wiz-step-less"
        } else {
            "wiz-step-more"
        }),
        Node {
            width: Val::Px(40.0),
            height: Val::Px(40.0),
            min_width: Val::Px(40.0),
            min_height: Val::Px(40.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
    let g = commands
        .spawn(glyph(
            if delta < 0 {
                glyphs::REMOVE
            } else {
                glyphs::ADD
            },
            20.0,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    commands.entity(button).add_child(g);
    button
}

/// One stepper: label above, `[-] value [+]` below. The buttons get the ids
/// `<id_prefix>.<field>.minus|plus` and the bundle `action(delta)`; the value text gets `marker`
/// (the owner keeps it current).
pub fn spawn_stepper<B: Bundle>(
    commands: &mut Commands,
    field: Field,
    value: String,
    marker: impl Bundle,
    id_prefix: &str,
    action: impl Fn(i32) -> B,
) -> Entity {
    let wrap = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            min_width: Val::Px(0.0),
            min_height: Val::Px(0.0),
            ..default()
        })
        .id();
    let label = commands
        .spawn((
            Text::new(field.label_key()),
            L10nKey(field.label_key()),
            TextColor::default(),
            TypeStyleToken(TypeScale::Label),
            ForegroundRole(ColorRole::OnSurfaceVariant),
        ))
        .id();
    let line = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    let minus = step_button(
        commands,
        format!("{id_prefix}.{}.minus", field.id()),
        -1,
        action(-1),
    );
    let val = commands
        .spawn((
            Text::new(value),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleMedium),
            ForegroundRole(ColorRole::OnSurface),
            marker,
            TextLayout::new(Justify::Center, LineBreak::NoWrap),
            Node {
                width: Val::Px(field.value_width()),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();
    let plus = step_button(
        commands,
        format!("{id_prefix}.{}.plus", field.id()),
        1,
        action(1),
    );
    commands.entity(line).add_children(&[minus, val, plus]);
    commands.entity(wrap).add_children(&[label, line]);
    wrap
}

/// A wrapping row of steppers.
pub fn spawn_stepper_row(commands: &mut Commands, steppers: &[Entity]) -> Entity {
    let row = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(16.0),
            row_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            min_width: Val::Px(0.0),
            ..default()
        })
        .id();
    commands.entity(row).add_children(steppers);
    row
}

/// A choice chip; `mark` is its check mark node (shown when chosen).
#[derive(Component, Debug, Clone, Copy)]
pub struct ChoiceMark {
    /// The check mark.
    pub mark: Entity,
}

/// Spawns the empty frame of a choice button (outlined, check mark first); the caller inserts its
/// action, adds its content and parents it. `node` gives the layout.
pub fn spawn_choice_frame(commands: &mut Commands, id: String, node: Node) -> Entity {
    let button = ButtonBuilder::outlined(String::new()).spawn(commands);
    let mark = commands
        .spawn((
            glyph(glyphs::CHECK, 18.0, ColorRole::OnSecondaryContainer),
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    let mut base = commands.entity(button);
    base.insert((
        UiId::new(id),
        ChoiceMark { mark },
        RadiusToken(RadiusLevel::Medium),
    ));
    base.queue(move |mut e: EntityWorldMut| {
        if let Some(mut n) = e.get_mut::<Node>() {
            n.flex_direction = node.flex_direction;
            n.align_items = node.align_items;
            n.padding = node.padding;
            n.row_gap = node.row_gap;
            n.column_gap = node.column_gap;
            n.min_height = node.min_height;
            n.width = node.width;
            n.justify_content = node.justify_content;
        }
    });
    commands.entity(button).add_child(mark);
    button
}

/// Gives a choice frame the chosen or the plain look (container fill plus check mark, never
/// colour alone).
pub fn show_chosen<F: bevy::ecs::query::QueryFilter>(
    commands: &mut Commands,
    nodes: &mut Query<&mut Node, F>,
    entity: Entity,
    mark: &ChoiceMark,
    on: bool,
) {
    let mut e = commands.entity(entity);
    if on {
        e.insert((
            opts_gui::interaction::state::Selected,
            BackgroundRole(ColorRole::SecondaryContainer),
        ));
    } else {
        e.remove::<opts_gui::interaction::state::Selected>();
        e.insert(BackgroundRole(ColorRole::Surface));
    }
    if let Ok(mut n) = nodes.get_mut(mark.mark) {
        n.display = if on { Display::Flex } else { Display::None };
    }
}

#[cfg(test)]
mod tests;
