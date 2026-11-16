//! The weather cards of the *When* step: the presets, and the disabled *Live weather* card.

use bevy::prelude::*;
use opts_gui::not_yet::NotAvailableYet;
use opts_gui::prelude::*;

use crate::new_game_screen::choice::{spawn_choice, ChoiceStep};
use crate::new_game_screen::planner::PlannerStep;
use crate::new_game_screen::ui::{column, glyph, keyed, plain, row, PlannerAction};
use crate::new_game_screen::weather::{presets, Label, MAX_WIZARD_PRESETS};
use crate::new_game_screen::BuildCtx;

/// A text that is a message key or data text.
fn label(commands: &mut Commands, label: Label, style: TypeScale, role: ColorRole) -> Entity {
    match label {
        Label::Key(key) => commands.spawn(keyed(key, style, role)).id(),
        Label::Text(text) => commands.spawn(plain(text, style, role)).id(),
    }
}

pub(super) fn weather_cards(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
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
