//! Step ① Where: the map list and the detail of the chosen map.

use bevy::prelude::*;
use opts_gui::components::combobox::ComboboxBuilder;
use opts_gui::l10n::L10nArgs;
use opts_gui::patterns::{
    glyphs, spawn_master_detail, ListEntry, MasterDetail, MasterDetailConfig,
};
use opts_gui::prelude::*;

use super::planner::{Note, PlannerStep};
use super::types::MapItem;
use super::ui::{
    action_button, column, glyph, keyed, keyed_args, plain, row, Emphasis, PlannerAction,
};
use super::BuildCtx;

/// Marks the root of a step's master-detail pattern.
#[derive(Component, Debug, Clone, Copy)]
pub struct StepList(pub PlannerStep);

/// The start point selector.
#[derive(Component, Debug, Clone, Copy)]
pub struct StartPointBox;

/// The hero image slot of a detail; `key` names the thumbnail.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct HeroImage {
    /// Thumbnail key (`map:<folder>`).
    pub key: String,
    /// Whether the slot is square (cropped from the middle) instead of 16:9.
    pub square: bool,
}

/// The map list entries.
pub fn map_entries(ctx: &BuildCtx) -> Vec<ListEntry> {
    ctx.catalogue
        .maps
        .iter()
        .map(|m| {
            ListEntry::new(m.id.clone(), m.friendly_name.clone())
                .with_subtitle(m.years.clone())
                .with_search_text(format!("{} {}", m.name, m.standard_depot))
                .with_glyph(glyphs::MAP)
                .with_thumb(format!("map:{}", m.id))
        })
        .collect()
}

/// Builds the step body: the pattern with the map list.
pub fn spawn_where(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let selected = ctx
        .catalogue
        .map_by_name(&ctx.setup.selected_map)
        .map(|m| m.id.clone());
    let mut model = MasterDetail::new(map_entries(ctx), selected);
    model.recents = ctx
        .history
        .history
        .recents
        .iter()
        .filter_map(|r| ctx.catalogue.map_by_name(&r.map))
        .map(|m| m.id.clone())
        .fold(Vec::new(), |mut acc, id| {
            if !acc.contains(&id) {
                acc.push(id);
            }
            acc
        });
    model.favourites = ctx.history.history.favourite_maps.clone();
    let parts = spawn_master_detail(
        commands,
        ctx.loc,
        MasterDetailConfig {
            prefix: "wizard.where",
            empty_key: "wiz-where-empty",
            search_key: "wiz-where-search",
        },
        model,
    );
    commands
        .entity(parts.root)
        .insert((StepList(PlannerStep::Where), Name::new("WhereStep")));
    commands.entity(parts.list).insert(DefaultFocus);
    parts.root
}

fn fact(commands: &mut Commands, key: &'static str, args: L10nArgs) -> Entity {
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
            super::thumbnails::HeroGlyph,
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
    kids.push(super::step_when::date_block(commands, ctx));
    if !map.description.trim().is_empty() {
        kids.push(description(commands, map));
    }
    commands.entity(detail).add_children(&kids);
}

fn missing_note(commands: &mut Commands, ctx: &BuildCtx) -> Option<Entity> {
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

fn facts_row(commands: &mut Commands, map: &MapItem) -> Entity {
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

fn start_point(commands: &mut Commands, ctx: &BuildCtx, map: &MapItem) -> Entity {
    let wrap = commands.spawn(column(6.0)).id();
    if map.entry_points.is_empty() {
        let none = commands
            .spawn(keyed(
                "wiz-where-no-starts",
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id();
        commands.entity(wrap).add_child(none);
        return wrap;
    }
    let label = commands
        .spawn(keyed(
            "wiz-where-start",
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let index = map
        .entry_points
        .iter()
        .position(|e| *e == ctx.setup.selected_entry_point)
        .unwrap_or(0);
    let combo = ComboboxBuilder::new(ctx.loc.t("wiz-where-start"))
        .with_options(map.entry_points.clone())
        .with_selected(index)
        .spawn(commands);
    commands
        .entity(combo)
        .insert((StartPointBox, UiId::new("wizard.where.start-point")));
    commands.entity(wrap).add_children(&[label, combo]);
    wrap
}

fn description(commands: &mut Commands, map: &MapItem) -> Entity {
    let wrap = commands.spawn(column(4.0)).id();
    let label = commands
        .spawn(keyed(
            "wiz-where-description",
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let text = description_box(commands, map.description.trim());
    commands.entity(wrap).add_children(&[label, text]);
    wrap
}

/// A text box of limited height that scrolls with the wheel (long descriptions).
pub fn description_box(commands: &mut Commands, text: &str) -> Entity {
    let boxed = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_height: Val::Px(150.0),
                padding: UiRect::all(Val::Px(8.0)),
                overflow: Overflow::scroll_y(),
                flex_shrink: 0.0,
                ..column(0.0)
            },
            BackgroundRole(ColorRole::SurfaceContainer),
            RadiusToken(RadiusLevel::Small),
            Name::new("DescriptionBox"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, boxed);
    let body = commands
        .spawn((
            plain(
                text.to_string(),
                TypeScale::BodyMedium,
                ColorRole::OnSurface,
            ),
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
        ))
        .id();
    commands.entity(boxed).add_child(body);
    boxed
}

/// The detail when the catalogue has no maps: what to do about it.
pub fn fill_empty_detail(commands: &mut Commands, detail: Entity, key: &'static str) {
    let text = commands
        .spawn(keyed(key, TypeScale::BodyLarge, ColorRole::OnSurface))
        .id();
    let button = action_button(
        commands,
        "wiz-open-settings",
        Emphasis::Tonal,
        PlannerAction::OpenSettings,
        "wizard.empty.open-settings".to_string(),
    );
    commands.entity(detail).add_children(&[text, button]);
}
