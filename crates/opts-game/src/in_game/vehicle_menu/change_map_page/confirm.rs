//! The confirmation page of a map change: what the change keeps and what it drops, the start
//! points, and the question that asks once.

use super::super::*;

use super::super::build::{button, label, scroll_list};
use crate::in_game::change_map::{ChangeMapError, ChangeMapPlan};

/// What the change keeps and what not: the vehicle, the time and the dropped options.
fn plan_labels(p: &mut ChildSpawnerCommands, plan: &ChangeMapPlan) {
    let vehicle = if plan.no_vehicle {
        "No vehicle (free camera).".to_owned()
    } else if plan.repaint.is_empty() {
        format!("Vehicle: {} is built again.", plan.vehicle_name)
    } else {
        format!(
            "Vehicle: {} ({}) is built again.",
            plan.vehicle_name, plan.repaint
        )
    };
    label(p, vehicle, 13.0, ColorRole::OnSurfaceVariant);
    label(
        p,
        format!(
            "Time: {:04}-{:02}-{:02} {:02}:{:02}, weather kept.",
            plan.date.year,
            plan.date.month,
            plan.date.day,
            (plan.seconds_of_day / 3600.0) as u32 % 24,
            (plan.seconds_of_day / 60.0) as u32 % 60
        ),
        13.0,
        ColorRole::OnSurfaceVariant,
    );
    if !plan.dropped.is_empty() {
        label(
            p,
            format!("Not carried over: {}.", plan.dropped.join(", ")),
            13.0,
            ColorRole::OnSurfaceVariant,
        );
    }
}

/// The chosen vehicle is gone: say so and offer to go on without one.
fn unavailable_notice(p: &mut ChildSpawnerCommands, v: &str) {
    label(
        p,
        format!("{v} is not installed any more and cannot be built on the other map."),
        13.0,
        ColorRole::Error,
    );
    let b = button(
        p,
        "Go on without a vehicle",
        false,
        Action::DriveWithoutVehicle,
    );
    p.commands()
        .entity(b)
        .insert(UiId::new("vehicle.map.without_vehicle"));
}

/// What the change does, and the question that asks once.
pub(in super::super) fn build_change_map_confirm(
    commands: &mut Commands,
    menu: &VehicleMenu,
    outcome: &Result<ChangeMapPlan, ChangeMapError>,
    entries: &[String],
    map_name: &str,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 80.0)
        .with_width_range(420.0, 640.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(content).with_children(|p| {
        label(p, format!("Change to {map_name}"), 22.0, ColorRole::Primary);
        match outcome {
            Ok(plan) => plan_labels(p, plan),
            Err(ChangeMapError::VehicleUnavailable(v)) => unavailable_notice(p, v),
            Err(e) => label(p, e.to_string(), 13.0, ColorRole::Error),
        }
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::OnSurfaceVariant);
        }
        if !entries.is_empty() {
            label(p, "Start point", 15.0, ColorRole::Primary);
            let chosen = outcome.as_ref().ok().map(|pl| pl.entry_point.as_str());
            scroll_list(p, 2, menu.scroll[2], Val::Percent(100.0), |list| {
                for (i, name) in entries.iter().enumerate() {
                    let b = button(
                        list,
                        name,
                        chosen == Some(name.as_str()),
                        Action::PickMapEntry(name.clone()),
                    );
                    list.commands()
                        .entity(b)
                        .insert(UiId::new(format!("vehicle.map.entry.{i}")));
                }
            });
        }
        if outcome.is_ok() {
            let go = button(p, "Change map", true, Action::ConfirmChangeMap);
            p.commands()
                .entity(go)
                .insert(UiId::new("vehicle.map.confirm"));
        }
        button(p, "Back", false, Action::Back);
    });
}
