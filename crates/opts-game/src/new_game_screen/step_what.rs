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

/// Message key of a vehicle class, `None` for unknown classes.
pub fn class_key(class: &str) -> Option<&'static str> {
    match class {
        "road" => Some("wiz-class-road"),
        "rail" => Some("wiz-class-rail"),
        "aircraft" => Some("wiz-class-aircraft"),
        _ => None,
    }
}

fn class_glyph(class: &str) -> &'static str {
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

/// The detail of *Drive without a vehicle*: what the session will be like.
pub fn fill_none_detail(commands: &mut Commands, detail: Entity) {
    let title = commands
        .spawn(keyed(
            "wiz-what-none",
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let body = commands
        .spawn(keyed(
            "wiz-what-none-body",
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    commands.entity(detail).add_children(&[title, body]);
}

/// The vehicle's picture slot, name, class and mass.
fn vehicle_header(commands: &mut Commands, vehicle: &VehicleItem) -> Entity {
    let header = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..super::ui::row(16.0)
        })
        .id();
    let thumb = commands
        .spawn((
            Node {
                width: Val::Px(96.0),
                height: Val::Px(96.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::Medium),
            HeroImage {
                key: format!("veh:{}", vehicle.id),
                square: true,
            },
        ))
        .id();
    let placeholder = commands
        .spawn((
            glyph(
                class_glyph(&vehicle.class),
                44.0,
                ColorRole::OnSurfaceVariant,
            ),
            super::thumbnails::HeroGlyph,
        ))
        .id();
    commands.entity(thumb).add_child(placeholder);
    let titles = commands.spawn(column(4.0)).id();
    let name = commands
        .spawn(plain(
            vehicle.full_name.clone(),
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let mut title_kids = vec![name];
    if let Some(k) = class_key(&vehicle.class) {
        title_kids.push(
            commands
                .spawn(keyed(k, TypeScale::BodyMedium, ColorRole::OnSurfaceVariant))
                .id(),
        );
    }
    if let Some(t) = vehicle.mass_tonnes {
        title_kids.push(
            commands
                .spawn(keyed_args(
                    "wiz-fact-mass",
                    L10nArgs::one("tonnes", format!("{t:.1}")),
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                ))
                .id(),
        );
    }
    commands.entity(titles).add_children(&title_kids);
    commands.entity(header).add_children(&[thumb, titles]);
    header
}

/// For a rail vehicle: the note and the button that opens the formation editor.
fn formation_entry(commands: &mut Commands, vehicle: &VehicleItem) -> Vec<Entity> {
    let mut out = Vec::new();
    if vehicle.class == "rail" {
        out.push(
            commands
                .spawn(keyed(
                    "wiz-what-formation-note",
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                ))
                .id(),
        );
        out.push(super::ui::action_button(
            commands,
            "wiz-what-formation",
            super::ui::Emphasis::Tonal,
            super::ui::PlannerAction::ComposeFormation,
            "wizard.what.formation".to_owned(),
        ));
    }
    out
}

/// The detail of a chosen vehicle.
pub fn fill_what_detail(
    commands: &mut Commands,
    detail: Entity,
    (loc, notes): (&Localization, &[Note]),
    setup: &NewGameSetup,
    liveries: &mut LiveryState,
    vehicle: &VehicleItem,
    (library, catalogue): (&ConfigLibrary, &super::types::GameCatalogue),
) {
    let header = vehicle_header(commands, vehicle);
    let mut kids = vec![header];
    if notes.contains(&Note::VehicleMissing) {
        kids.push(
            commands
                .spawn(keyed(
                    "wiz-what-remembered-missing",
                    TypeScale::BodyMedium,
                    ColorRole::Warning,
                ))
                .id(),
        );
    }
    if !vehicle.description.trim().is_empty() {
        kids.push(super::step_where::description_box(
            commands,
            vehicle.description.trim(),
        ));
    }
    kids.push(spawn_livery_section(commands, liveries, setup, vehicle));
    let repaints = liveries
        .repaints
        .clone()
        .filter(|_| liveries.vehicle == vehicle.id);
    kids.push(spawn_configs_section(
        commands,
        loc,
        library,
        catalogue,
        setup,
        vehicle,
        repaints.as_ref(),
    ));
    kids.extend(formation_entry(commands, vehicle));
    kids.push(super::ui::action_button(
        commands,
        "wiz-what-none",
        super::ui::Emphasis::Quiet,
        super::ui::PlannerAction::NoVehicle,
        "wizard.what.none".to_owned(),
    ));
    commands.entity(detail).add_children(&kids);
}
