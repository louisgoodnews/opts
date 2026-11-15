//! Step *Review*: one card with the choices (and the trip, when runs are offered) and a *Change*
//! link each.

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

mod rows;
mod summary;

use rows::{what_row, where_row};
use summary::{trip_row, when_row};
use rows::review_row;

/// Message key of the reason a drive cannot start.
pub fn reason_key(reason: Invalid) -> &'static str {
    match reason {
        Invalid::NoMap => "wiz-reason-no-map",
        Invalid::NoStartPoint => "wiz-reason-no-start",
        Invalid::NoVehicle => "wiz-reason-no-vehicle",
    }
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
    let where_row = where_row(commands, ctx);
    let what_row = what_row(commands, ctx);
    let when_row = when_row(commands, ctx);
    let mut rows = vec![where_row, what_row, when_row];
    if let Some(trip) = trip_row(commands, ctx) {
        rows.push(trip);
    }
    commands.entity(card).add_children(&rows);
    let mut kids = vec![title, card, super::step_load::spawn_load_row(commands)];
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
