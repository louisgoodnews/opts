//! Step ③ When: quick times, fixed-size time steppers, weather presets. The date steppers (built
//! here with [`date_block`]) are on the map step, the one place where the date is chosen.

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;
use opts_gui::not_yet::NotAvailableYet;
use opts_gui::prelude::*;

use super::choice::{spawn_choice, ChoiceStep};
use super::planner::{
    era_hint, month_key, Field, PlannerState, PlannerStep, TimeSource, QUICK_TIMES,
};
use super::types::NewGameSetup;
use super::ui::{column, glyph, keyed, keyed_args, plain, row, Act, PlannerAction};
use super::weather::{presets, Label, MAX_WIZARD_PRESETS};
use super::BuildCtx;
use crate::shell::calendar;

/// A value text of a stepper; [`refresh_when`] keeps it current.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhenValue(pub Field);

/// The era hint line (shown only outside the map's years).
#[derive(Component, Debug, Clone, Copy)]
pub struct EraHint;

/// A line under the time that says where a default time comes from; [`refresh_when`] shows the
/// one that matches [`PlannerState::time_source`] and none once the user has set the time.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSourceLabel(pub TimeSource);

/// One stepper of the plan (`shell::calendar`): ids `wizard.<step>.<field>.minus|plus`.
pub fn stepper(commands: &mut Commands, field: Field, value: String, id_prefix: &str) -> Entity {
    calendar::spawn_stepper(commands, field, value, WhenValue(field), id_prefix, |d| {
        Act(PlannerAction::Step(field, d))
    })
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

fn time_block(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
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

/// A text that is a message key or data text.
fn label(commands: &mut Commands, label: Label, style: TypeScale, role: ColorRole) -> Entity {
    match label {
        Label::Key(key) => commands.spawn(keyed(key, style, role)).id(),
        Label::Text(text) => commands.spawn(plain(text, style, role)).id(),
    }
}

fn weather_cards(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let wrap = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Stretch,
            ..row(12.0)
        })
        .id();
    for p in presets(ctx.weather, ctx.loc)
        .into_iter()
        .take(MAX_WIZARD_PRESETS)
    {
        let card = spawn_choice(
            commands,
            PlannerAction::Weather(p.id.clone()),
            format!("wizard.when.weather.{}", opts_gui::ui_id::slug(&p.id)),
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
        let name = label(
            commands,
            p.name,
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        );
        let effect = label(
            commands,
            p.effect,
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        );
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
    let dt = time_block(commands, ctx);
    let w_title = section_title(commands, "wiz-when-weather");
    let weather = weather_cards(commands, ctx);
    commands
        .entity(body)
        .add_children(&[quick_title, quick, dt_title, dt, w_title, weather]);
    super::ui::no_shrink(
        commands,
        &[quick_title, quick, dt_title, dt, w_title, weather],
    );
    body
}

/// Parts of the step that were just spawned and need their first refresh.
type NewWhenParts = Or<(Added<WhenValue>, Added<TimeSourceLabel>)>;

/// Keeps the stepper values and the era hint current.
pub fn refresh_when(
    setup: Res<NewGameSetup>,
    state: Res<PlannerState>,
    lookup: super::TextsAndCatalogue,
    mut values: Query<(&WhenValue, &mut Text)>,
    mut hints: Query<(&mut Node, &mut L10nArgs), With<EraHint>>,
    mut sources: Query<(&TimeSourceLabel, &mut Node), Without<EraHint>>,
    added: Query<(), NewWhenParts>,
) {
    let super::TextsAndCatalogue { loc, catalogue } = lookup;
    if !setup.is_changed() && !state.is_changed() && added.is_empty() {
        return;
    }
    for (label, mut node) in &mut sources {
        let shown = if state.time_source == Some(label.0) && !state.time_chosen {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != shown {
            node.display = shown;
        }
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
