//! Step ④ Review: one card with the three choices and a *Change* link each.

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;
use opts_gui::patterns::glyphs;
use opts_gui::prelude::*;

use super::planner::{validate, Invalid, PlannerStep};
use super::step_where::HeroImage;
use super::texts::{date_text, livery_name, vehicle_line, weather_name};
use super::ui::{
    action_button, column, glyph, keyed, keyed_args, plain, row, Emphasis, PlannerAction,
};
use super::BuildCtx;

/// Message key of the reason a drive cannot start.
pub fn reason_key(reason: Invalid) -> &'static str {
    match reason {
        Invalid::NoMap => "wiz-reason-no-map",
        Invalid::NoStartPoint => "wiz-reason-no-start",
        Invalid::NoVehicle => "wiz-reason-no-vehicle",
    }
}

fn review_row(
    commands: &mut Commands,
    step: PlannerStep,
    glyph_code: &'static str,
    hero: Option<String>,
    lines: Vec<Entity>,
) -> Entity {
    let line = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            align_items: AlignItems::Center,
            ..row(16.0)
        })
        .id();
    let thumb = commands
        .spawn((
            Node {
                width: Val::Px(72.0),
                height: Val::Px(72.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::Medium),
        ))
        .id();
    if let Some(key) = hero {
        commands
            .entity(thumb)
            .insert(HeroImage { key, square: true });
    }
    let g = commands
        .spawn((
            glyph(glyph_code, 32.0, ColorRole::OnSurfaceVariant),
            super::thumbnails::HeroGlyph,
        ))
        .id();
    commands.entity(thumb).add_child(g);
    let texts = commands
        .spawn(Node {
            flex_grow: 1.0,
            ..column(2.0)
        })
        .id();
    commands.entity(texts).add_children(&lines);
    let change = action_button(
        commands,
        "wiz-change",
        Emphasis::Quiet,
        PlannerAction::GoStep(step),
        format!("wizard.review.change.{}", step.id()),
    );
    commands.entity(line).add_children(&[thumb, texts, change]);
    line
}

/// Builds the step body.
pub fn spawn_review(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let s = ctx.setup;
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                overflow: Overflow::scroll_y(),
                ..column(16.0)
            },
            Name::new("ReviewStep"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, body);
    let title = commands
        .spawn(keyed(
            "wiz-review-title",
            TypeScale::Headline,
            ColorRole::OnSurface,
        ))
        .id();
    let card = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(760.0),
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..column(16.0)
            },
            BackgroundRole(ColorRole::SurfaceContainerLow),
            BorderRole(ColorRole::OutlineVariant),
            RadiusToken(RadiusLevel::Large),
            Name::new("ReviewCard"),
        ))
        .id();
    let map = ctx.catalogue.map_by_name(&s.selected_map);
    let where_lines = vec![
        commands
            .spawn(plain(
                s.selected_map.clone(),
                TypeScale::TitleMedium,
                ColorRole::OnSurface,
            ))
            .id(),
        commands
            .spawn(plain(
                s.selected_entry_point.clone(),
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id(),
    ];
    let where_row = review_row(
        commands,
        PlannerStep::Where,
        glyphs::MAP,
        map.map(|m| format!("map:{}", m.id)),
        where_lines,
    );
    let vehicle = ctx
        .catalogue
        .find_vehicle(&s.selected_vehicle)
        .filter(|_| !s.no_vehicle);
    let mut what_lines = vec![commands
        .spawn(plain(
            vehicle_line(ctx.loc, ctx.catalogue, s),
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id()];
    let livery = livery_name(ctx.catalogue, s);
    if !livery.is_empty() {
        what_lines.push(
            commands
                .spawn(plain(
                    livery,
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                ))
                .id(),
        );
    }
    if !s.custom_setvars.is_empty() {
        what_lines.push(
            commands
                .spawn(keyed_args(
                    "wiz-what-options-changed",
                    L10nArgs::one("count", s.custom_setvars.len()),
                    TypeScale::Label,
                    ColorRole::OnSurfaceVariant,
                ))
                .id(),
        );
    }
    let what_row = review_row(
        commands,
        PlannerStep::What,
        glyphs::BUS,
        vehicle.map(|v| format!("veh:{}", v.id)),
        what_lines,
    );
    let when_lines = vec![
        commands
            .spawn(plain(
                format!(
                    "{} {}",
                    date_text(ctx.loc, s),
                    super::planner::clock_text(s)
                ),
                TypeScale::TitleMedium,
                ColorRole::OnSurface,
            ))
            .id(),
        commands
            .spawn(plain(
                weather_name(ctx.loc, ctx.weather, s),
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id(),
    ];
    let when_row = review_row(
        commands,
        PlannerStep::When,
        glyphs::SCHEDULE,
        None,
        when_lines,
    );
    commands
        .entity(card)
        .add_children(&[where_row, what_row, when_row]);
    let mut kids = vec![title, card];
    if let Err(reason) = validate(s, ctx.catalogue) {
        kids.push(
            commands
                .spawn(keyed(
                    reason_key(reason),
                    TypeScale::BodyMedium,
                    ColorRole::Error,
                ))
                .id(),
        );
    }
    commands.entity(body).add_children(&kids);
    super::ui::no_shrink(commands, &kids);
    body
}
