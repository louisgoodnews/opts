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

mod detail;
mod start;

pub use detail::fill_where_detail;
pub use start::description_box;
use start::{description, start_point};

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
