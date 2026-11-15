//! The detail of a chosen vehicle in step What: its header, notes, livery, configurations and the composer entry.

use super::*;

/// The vehicle's picture slot, name, class and mass.
pub(super) fn vehicle_header(commands: &mut Commands, vehicle: &VehicleItem) -> Entity {
    let header = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..super::super::ui::row(16.0)
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
            super::super::thumbnails::HeroGlyph,
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

/// The detail of a chosen vehicle.
pub fn fill_what_detail(
    commands: &mut Commands,
    detail: Entity,
    (loc, notes): (&Localization, &[Note]),
    setup: &NewGameSetup,
    liveries: &mut LiveryState,
    vehicle: &VehicleItem,
    (library, catalogue): (&ConfigLibrary, &super::super::types::GameCatalogue),
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
        kids.push(super::super::step_where::description_box(
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
    kids.push(super::super::ui::action_button(
        commands,
        "wiz-what-none",
        super::super::ui::Emphasis::Quiet,
        super::super::ui::PlannerAction::NoVehicle,
        "wizard.what.none".to_owned(),
    ));
    commands.entity(detail).add_children(&kids);
}
