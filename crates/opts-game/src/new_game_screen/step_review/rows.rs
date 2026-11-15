//! The summary rows of step Review for the map, the vehicle and their review card row.

use super::*;

pub(super) fn review_row(
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
            super::super::thumbnails::HeroGlyph,
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

/// The *Where* row: the map and the start point (the first stop with a trip).
pub(super) fn where_row(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let s = ctx.setup;
    let map = ctx.catalogue.map_by_name(&s.selected_map);
    // With a trip the drive starts at its first stop, not at the start point.
    let start = s
        .trip
        .as_ref()
        .map_or_else(|| s.selected_entry_point.clone(), |t| t.stop_name.clone());
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
                start,
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id(),
    ];
    review_row(
        commands,
        PlannerStep::Where,
        glyphs::MAP,
        map.map(|m| format!("map:{}", m.id)),
        where_lines,
    )
}

/// The *What* row: the vehicle, its repaint and how many options were changed.
pub(super) fn what_row(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let s = ctx.setup;
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
    review_row(
        commands,
        PlannerStep::What,
        glyphs::BUS,
        vehicle.map(|v| format!("veh:{}", v.id)),
        what_lines,
    )
}
