//! Step ③ When: quick times, fixed-size date and time steppers, weather presets.

use bevy::prelude::*;
use opts_gui::l10n::{L10nArgs, Localization};
use opts_gui::not_yet::NotAvailableYet;
use opts_gui::prelude::*;

use super::choice::{spawn_choice, ChoiceStep};
use super::planner::{era_hint, month_key, Field, PlannerStep, QUICK_TIMES};
use super::types::NewGameSetup;
use super::ui::{column, glyph, keyed, keyed_args, plain, row, Act, PlannerAction};
use super::weather::PRESETS;
use super::BuildCtx;

/// A value text of a stepper; [`refresh_when`] keeps it current.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhenValue(pub Field);

/// The era hint line (shown only outside the map's years).
#[derive(Component, Debug, Clone, Copy)]
pub struct EraHint;

/// Width of the value box of a stepper, per field.
fn value_width(field: Field) -> f32 {
    match field {
        Field::Day | Field::Hour | Field::Minute => 48.0,
        Field::Month => 96.0,
        Field::Year => 64.0,
    }
}

fn label_key(field: Field) -> &'static str {
    match field {
        Field::Day => "wiz-when-day",
        Field::Month => "wiz-when-month",
        Field::Year => "wiz-when-year",
        Field::Hour => "wiz-when-hour",
        Field::Minute => "wiz-when-minute",
    }
}

fn step_button(commands: &mut Commands, field: Field, delta: i32) -> Entity {
    let sign = if delta < 0 { "minus" } else { "plus" };
    let button = ButtonBuilder::text(String::new()).spawn(commands);
    commands.entity(button).insert((
        Act(PlannerAction::Step(field, delta)),
        UiId::new(format!("wizard.when.{}.{sign}", label_id(field))),
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
                opts_gui::patterns::glyphs::REMOVE
            } else {
                opts_gui::patterns::glyphs::ADD
            },
            20.0,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    commands.entity(button).add_child(g);
    button
}

fn label_id(field: Field) -> &'static str {
    match field {
        Field::Day => "day",
        Field::Month => "month",
        Field::Year => "year",
        Field::Hour => "hour",
        Field::Minute => "minute",
    }
}

/// One stepper: label above, `[-] value [+]` below.
fn stepper(commands: &mut Commands, field: Field, value: String) -> Entity {
    let wrap = commands.spawn(column(4.0)).id();
    let label = commands
        .spawn(keyed(
            label_key(field),
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let line = commands.spawn(row(8.0)).id();
    let minus = step_button(commands, field, -1);
    let val = commands
        .spawn((
            plain(value, TypeScale::TitleMedium, ColorRole::OnSurface),
            WhenValue(field),
            TextLayout::new(Justify::Center, LineBreak::NoWrap),
            Node {
                width: Val::Px(value_width(field)),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();
    let plus = step_button(commands, field, 1);
    commands.entity(line).add_children(&[minus, val, plus]);
    commands.entity(wrap).add_children(&[label, line]);
    wrap
}

fn section_title(commands: &mut Commands, key: &'static str) -> Entity {
    commands
        .spawn(keyed(key, TypeScale::TitleMedium, ColorRole::OnSurface))
        .id()
}

fn quick_chips(commands: &mut Commands) -> Entity {
    let line = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            ..row(8.0)
        })
        .id();
    for (i, q) in QUICK_TIMES.iter().enumerate() {
        let chip = spawn_choice(
            commands,
            PlannerAction::Quick(i),
            format!("wizard.when.quick.{i}"),
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                ..default()
            },
        );
        let text = commands
            .spawn(keyed(q.key, TypeScale::BodyMedium, ColorRole::OnSurface))
            .id();
        commands
            .entity(chip)
            .insert((
                ChoiceStep(PlannerStep::When),
                RadiusToken(RadiusLevel::Full),
            ))
            .insert_children(1, &[text]);
        commands.entity(line).add_child(chip);
    }
    line
}

fn date_time_block(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let s = ctx.setup;
    let block = commands
        .spawn((
            Node {
                max_width: Val::Px(560.0),
                ..column(12.0)
            },
            Name::new("DateTimeBlock"),
        ))
        .id();
    let date = commands
        .spawn(Node {
            column_gap: Val::Px(16.0),
            flex_wrap: FlexWrap::Wrap,
            ..row(16.0)
        })
        .id();
    let day = stepper(commands, Field::Day, s.day.to_string());
    let month = stepper(commands, Field::Month, ctx.loc.t(month_key(s.month)));
    let year = stepper(commands, Field::Year, s.year.to_string());
    commands.entity(date).add_children(&[day, month, year]);
    let time = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            ..row(16.0)
        })
        .id();
    let hour = stepper(commands, Field::Hour, format!("{:02}", s.hour));
    let minute = stepper(commands, Field::Minute, format!("{:02}", s.minute));
    commands.entity(time).add_children(&[hour, minute]);
    let mut kids = vec![date, time];
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
    kids.push(hint_node);
    commands.entity(block).add_children(&kids);
    block
}

fn weather_cards(commands: &mut Commands) -> Entity {
    let wrap = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Stretch,
            ..row(12.0)
        })
        .id();
    for p in PRESETS {
        let card = spawn_choice(
            commands,
            PlannerAction::Weather(p.id),
            format!("wizard.when.weather.{}", opts_gui::ui_id::slug(p.id)),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                width: Val::Px(236.0),
                min_height: Val::Px(64.0),
                ..default()
            },
        );
        let g = commands
            .spawn(glyph(p.glyph, 28.0, ColorRole::OnSurfaceVariant))
            .id();
        let texts = commands
            .spawn(Node {
                flex_grow: 1.0,
                ..column(2.0)
            })
            .id();
        let name = commands
            .spawn(keyed(
                p.name_key,
                TypeScale::TitleMedium,
                ColorRole::OnSurface,
            ))
            .id();
        let effect = commands
            .spawn(keyed(
                p.effect_key,
                TypeScale::Label,
                ColorRole::OnSurfaceVariant,
            ))
            .id();
        commands.entity(texts).add_children(&[name, effect]);
        commands
            .entity(card)
            .insert(ChoiceStep(PlannerStep::When))
            .insert_children(1, &[g, texts]);
        commands.entity(wrap).add_child(card);
    }
    append_live_card(commands, wrap);
    wrap
}

/// Appends the *Live weather* card: visible, disabled, marked, without any numbers.
fn append_live_card(commands: &mut Commands, wrap: Entity) {
    let card = ButtonBuilder::outlined(String::new()).spawn(commands);
    let g = commands
        .spawn(glyph(
            opts_gui::patterns::glyphs::PUBLIC,
            28.0,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let name = commands
        .spawn(keyed(
            "wiz-weather-live",
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id();
    commands
        .entity(card)
        .insert((
            NotAvailableYet::new(&opts_gui::not_yet::LIVE_WEATHER),
            UiId::new("wizard.when.weather.live"),
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                width: Val::Px(236.0),
                min_height: Val::Px(64.0),
                row_gap: Val::Px(2.0),
                ..default()
            },
        ))
        .add_children(&[g, name]);
    commands.entity(wrap).add_child(card);
}

/// Builds the step body.
pub fn spawn_when(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                padding: UiRect::all(Val::Px(4.0)),
                overflow: Overflow::scroll_y(),
                ..column(16.0)
            },
            Name::new("WhenStep"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, body);
    let quick_title = section_title(commands, "wiz-when-quick");
    let quick = quick_chips(commands);
    let dt_title = section_title(commands, "wiz-when-exact");
    let dt = date_time_block(commands, ctx);
    let w_title = section_title(commands, "wiz-when-weather");
    let weather = weather_cards(commands);
    commands
        .entity(body)
        .add_children(&[quick_title, quick, dt_title, dt, w_title, weather]);
    super::ui::no_shrink(
        commands,
        &[quick_title, quick, dt_title, dt, w_title, weather],
    );
    body
}

/// Keeps the stepper values and the era hint current.
pub fn refresh_when(
    setup: Res<NewGameSetup>,
    loc: Res<Localization>,
    catalogue: Res<super::types::GameCatalogue>,
    mut values: Query<(&WhenValue, &mut Text)>,
    mut hints: Query<(&mut Node, &mut L10nArgs), With<EraHint>>,
    added: Query<(), Added<WhenValue>>,
) {
    if !setup.is_changed() && added.is_empty() {
        return;
    }
    for (v, mut text) in &mut values {
        let new = match v.0 {
            Field::Day => setup.day.to_string(),
            Field::Month => loc.t(month_key(setup.month)),
            Field::Year => setup.year.to_string(),
            Field::Hour => format!("{:02}", setup.hour),
            Field::Minute => format!("{:02}", setup.minute),
        };
        if text.0 != new {
            text.0 = new;
        }
    }
    let hint = catalogue
        .map_by_name(&setup.selected_map)
        .and_then(|m| era_hint(&m.years, setup.year));
    for (mut node, mut args) in &mut hints {
        node.display = if hint.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some((from, to)) = hint {
            *args = L10nArgs(vec![("from".into(), from.into()), ("to".into(), to.into())]);
        }
    }
}
