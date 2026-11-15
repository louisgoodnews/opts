//! Step ② What: the vehicle list and the detail of the chosen vehicle.

use bevy::prelude::*;
use opts_gui::l10n::{L10nArgs, Localization};
use opts_gui::patterns::{
    glyphs, spawn_master_detail, Chip, ListEntry, MasterDetail, MasterDetailConfig,
};
use opts_gui::prelude::*;

use super::configs::{spawn_configs_section, ConfigLibrary};
use super::livery::{spawn_livery_section, LiveryState};
use super::planner::{Note, PlannerStep};
use super::step_where::{HeroImage, StepList};
use super::types::{NewGameSetup, VehicleItem};
use super::ui::{column, glyph, keyed, keyed_args, plain};
use super::BuildCtx;

mod detail;
mod none;

pub use detail::fill_what_detail;
pub use none::fill_none_detail;
use none::formation_entry;

/// Message key of a vehicle class, `None` for unknown classes.
pub fn class_key(class: &str) -> Option<&'static str> {
    match class {
        "road" => Some("wiz-class-road"),
        "rail" => Some("wiz-class-rail"),
        "aircraft" => Some("wiz-class-aircraft"),
        _ => None,
    }
}

pub(super) fn class_glyph(class: &str) -> &'static str {
    match class {
        "road" => glyphs::BUS,
        "rail" => glyphs::TRAIN,
        "aircraft" => glyphs::FLIGHT,
        _ => glyphs::IMAGE,
    }
}

/// Id of the pinned list entry *Drive without a vehicle* (never a catalogue id: those are file
/// stems and cannot contain a colon).
pub const NONE_ID: &str = ":none";

/// The vehicle list entries: *Drive without a vehicle* first, then the vehicles by name and class,
/// never file names.
pub fn vehicle_entries(ctx: &BuildCtx) -> Vec<ListEntry> {
    let none = ListEntry::new(NONE_ID, ctx.loc.t("wiz-what-none"))
        .with_glyph(glyphs::MAP)
        .with_subtitle(ctx.loc.t("wiz-what-none-sub"));
    std::iter::once(none)
        .chain(ctx.catalogue.vehicles.iter().map(|v| {
            let mut e = ListEntry::new(v.id.clone(), v.full_name.clone())
                .with_glyph(class_glyph(&v.class))
                .with_thumb(format!("veh:{}", v.id))
                .with_search_text(format!("{} {}", v.manufacturer, v.default_livery));
            if let Some(k) = class_key(&v.class) {
                e = e.with_subtitle(ctx.loc.t(k)).with_tags([v.class.clone()]);
            }
            e
        }))
        .collect()
}

fn class_chips(ctx: &BuildCtx) -> Vec<Chip> {
    let mut chips = Vec::new();
    for v in &ctx.catalogue.vehicles {
        if let Some(k) = class_key(&v.class) {
            if !chips.iter().any(|c: &Chip| c.tag == v.class) {
                chips.push(Chip {
                    tag: v.class.clone(),
                    label: ctx.loc.t(k),
                });
            }
        }
    }
    chips
}

/// Builds the step body: the pattern with the vehicle list.
pub fn spawn_what(commands: &mut Commands, ctx: &BuildCtx) -> Entity {
    let selected = if ctx.setup.no_vehicle {
        Some(NONE_ID.to_owned())
    } else {
        ctx.catalogue
            .find_vehicle(&ctx.setup.selected_vehicle)
            .map(|v| v.id.clone())
    };
    let mut model = MasterDetail::new(vehicle_entries(ctx), selected);
    model.chips = class_chips(ctx);
    model.recents = ctx
        .history
        .history
        .recents
        .iter()
        .filter_map(|r| ctx.catalogue.find_vehicle(&r.vehicle))
        .map(|v| v.id.clone())
        .fold(Vec::new(), |mut acc, id| {
            if !acc.contains(&id) {
                acc.push(id);
            }
            acc
        });
    model.favourites = ctx.history.history.favourite_vehicles.clone();
    let parts = spawn_master_detail(
        commands,
        ctx.loc,
        MasterDetailConfig {
            prefix: "wizard.what",
            empty_key: "wiz-what-empty",
            search_key: "wiz-what-search",
        },
        model,
    );
    commands
        .entity(parts.root)
        .insert((StepList(PlannerStep::What), Name::new("WhatStep")));
    commands.entity(parts.list).insert(DefaultFocus);
    parts.root
}
