//! UI builders for the optional *Trip* step in the *Plan a drive* wizard.

use super::*;

/// The list entries: *No trip* first, then the runs.
fn trip_entries(ctx: &BuildCtx) -> Vec<ListEntry> {
    let mut out = vec![ListEntry::new(NO_TRIP_ID, ctx.loc.t("wiz-trip-none"))
        .with_subtitle(ctx.loc.t("wiz-trip-none-hint"))
        .with_glyph(glyphs::MAP)];
    out.extend(ctx.trips.trips.iter().map(|t| {
        let c = &t.choice;
        ListEntry::new(c.id(), format!("{}  {}", c.depart_text(), c.title()))
            .with_subtitle(format!("{} · {}", c.stop_name, t.tour_name))
            .with_search_text(format!("{} {} {}", c.line, c.destination, c.stop_name))
            .with_glyph(glyphs::SCHEDULE)
    }));
    out
}

/// Builds the step body: the pattern with the runs.
pub fn spawn_trip(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let selected = Some(
        ctx.setup
            .trip
            .as_ref()
            .map_or_else(|| NO_TRIP_ID.to_string(), TripChoice::id),
    );
    let mut model = MasterDetail::new(trip_entries(ctx), selected);
    model.no_stars = true;
    let parts = spawn_master_detail(
        commands,
        ctx.loc,
        MasterDetailConfig {
            prefix: "wizard.trip",
            empty_key: "wiz-trip-empty",
            search_key: "wiz-trip-search",
        },
        model,
    );
    commands
        .entity(parts.root)
        .insert((StepList(PlannerStep::Trip), Name::new("TripStep")));
    commands.entity(parts.list).insert(DefaultFocus);
    parts.root
}

fn line(commands: &mut Commands, text: String, scale: TypeScale, role: ColorRole) -> Entity {
    commands.spawn(plain(text, scale, role)).id()
}

/// The detail of a run.
pub fn fill_trip_detail(commands: &mut Commands, detail: Entity, trip: &TripOption) {
    let c = &trip.choice;
    let kids = vec![
        line(
            commands,
            c.title(),
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ),
        commands
            .spawn(keyed_args(
                "wiz-trip-departs",
                opts_gui::l10n::L10nArgs(vec![
                    ("time".into(), c.depart_text().into()),
                    ("stop".into(), c.stop_name.clone().into()),
                ]),
                TypeScale::BodyLarge,
                ColorRole::OnSurface,
            ))
            .id(),
        commands
            .spawn(keyed_args(
                "wiz-trip-stops",
                opts_gui::l10n::L10nArgs(vec![
                    ("count".into(), trip.stops.into()),
                    ("last".into(), trip.last_stop.clone().into()),
                ]),
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id(),
        commands
            .spawn(keyed_args(
                "wiz-trip-tour",
                opts_gui::l10n::L10nArgs::one("tour", trip.tour_name.clone()),
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id(),
        commands
            .spawn(keyed(
                "wiz-trip-start-hint",
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id(),
    ];
    let col = commands.spawn(column(8.0)).id();
    commands.entity(col).add_children(&kids);
    commands.entity(detail).add_child(col);
}

/// The detail of *No trip*.
pub fn fill_no_trip_detail(commands: &mut Commands, detail: Entity, entry_point: &str) {
    let kids = vec![
        commands
            .spawn(keyed(
                "wiz-trip-none",
                TypeScale::TitleLarge,
                ColorRole::OnSurface,
            ))
            .id(),
        commands
            .spawn(keyed_args(
                "wiz-trip-none-detail",
                opts_gui::l10n::L10nArgs::one("start", entry_point.to_string()),
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id(),
    ];
    let col = commands.spawn(column(8.0)).id();
    commands.entity(col).add_children(&kids);
    commands.entity(detail).add_child(col);
}
