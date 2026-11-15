//! The trip and the date and weather summary rows of step Review.

use super::*;

/// The trip row of the summary, only when timetable runs are offered: the chosen run, or "none".
pub(super) fn trip_row(commands: &mut Commands, ctx: &BuildCtx) -> Option<Entity> {
    if ctx.trips.trips.is_empty() {
        return None;
    }
    let trip_lines = match &ctx.setup.trip {
        Some(t) => vec![
            commands
                .spawn(plain(
                    format!("{}  {}", t.depart_text(), t.title()),
                    TypeScale::TitleMedium,
                    ColorRole::OnSurface,
                ))
                .id(),
            commands
                .spawn(keyed_args(
                    "wiz-review-trip-from",
                    L10nArgs::one("stop", t.stop_name.clone()),
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                ))
                .id(),
        ],
        None => vec![commands
            .spawn(keyed(
                "wiz-trip-none",
                TypeScale::TitleMedium,
                ColorRole::OnSurface,
            ))
            .id()],
    };
    Some(review_row(
        commands,
        PlannerStep::Trip,
        glyphs::SCHEDULE,
        None,
        trip_lines,
    ))
}

/// The *When* row: date, time and weather.
pub(super) fn when_row(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let s = ctx.setup;
    let when_lines = vec![
        commands
            .spawn(plain(
                format!(
                    "{} {}",
                    date_text(ctx.loc, s),
                    super::super::planner::clock_text(s)
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
    review_row(
        commands,
        PlannerStep::When,
        glyphs::SCHEDULE,
        None,
        when_lines,
    )
}
