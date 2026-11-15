//! The detail of a chosen map in step Where: the hero, the facts and the remembered-map note.

use super::*;

pub(super) fn fact(commands: &mut Commands, key: &'static str, args: L10nArgs) -> Entity {
    commands
        .spawn(keyed_args(
            key,
            args,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id()
}

/// The detail of a chosen map.
pub fn fill_where_detail(commands: &mut Commands, detail: Entity, ctx: &BuildCtx, map: &MapItem) {
    let hero = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                aspect_ratio: Some(16.0 / 9.0),
                max_height: Val::Px(220.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::Medium),
            HeroImage {
                key: format!("map:{}", map.id),
                square: false,
            },
            Name::new("MapHero"),
        ))
        .id();
    let hero_glyph = commands
        .spawn((
            glyph(glyphs::MAP, 48.0, ColorRole::OnSurfaceVariant),
            super::super::thumbnails::HeroGlyph,
        ))
        .id();
    commands.entity(hero).add_child(hero_glyph);
    let title = commands
        .spawn(plain(
            map.friendly_name.clone(),
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let mut kids = vec![hero, title];
    if let Some(note) = missing_note(commands, ctx) {
        kids.push(note);
    }
    kids.push(facts_row(commands, map));
    kids.push(start_point(commands, ctx, map));
    kids.push(action_button(
        commands,
        "map-config-open",
        Emphasis::Tonal,
        PlannerAction::OpenMapConfig,
        "wizard.where.map-config".to_owned(),
    ));
    kids.push(super::super::step_when::date_block(commands, ctx));
    if !map.description.trim().is_empty() {
        kids.push(description(commands, map));
    }
    commands.entity(detail).add_children(&kids);
}

pub(super) fn missing_note(commands: &mut Commands, ctx: &BuildCtx) -> Option<Entity> {
    ctx.notes.contains(&Note::MapMissing).then(|| {
        commands
            .spawn(keyed(
                "wiz-where-remembered-missing",
                TypeScale::BodyMedium,
                ColorRole::Warning,
            ))
            .id()
    })
}

pub(super) fn facts_row(commands: &mut Commands, map: &MapItem) -> Entity {
    let line = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(16.0),
            ..row(16.0)
        })
        .id();
    let mut facts = Vec::new();
    if !map.years.trim().is_empty() {
        facts.push(fact(
            commands,
            "wiz-fact-years",
            L10nArgs::one("years", map.years.clone()),
        ));
    }
    if map.tile_count > 0 {
        facts.push(fact(
            commands,
            "wiz-fact-tiles",
            L10nArgs::one("count", map.tile_count),
        ));
    }
    if !map.entry_points.is_empty() {
        facts.push(fact(
            commands,
            "wiz-fact-starts",
            L10nArgs::one("count", map.entry_points.len()),
        ));
    }
    if !map.standard_depot.trim().is_empty() {
        facts.push(fact(
            commands,
            "wiz-fact-depot",
            L10nArgs::one("depot", map.standard_depot.clone()),
        ));
    }
    commands.entity(line).add_children(&facts);
    line
}
