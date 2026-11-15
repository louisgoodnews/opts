//! The *Setvars* page of the Alt menu (M-0110): the equipment options of the active vehicle with
//! a toggle or stepper each. A change reaches the vehicle's scripts at once and is kept on its
//! session record, so the session save and *Save current vehicle* carry it.
//!
//! Whether a script reads an option again after it started is not known per option
//! (`docs/src/formats/text/vehicles/setvars.md`, Q-SV02): the page says so instead of promising
//! an effect, and a configuration spawned from the *Configurations* page applies every option
//! from the start.

use super::build::{button, label, scroll_list, setvar_rows};
use super::*;
use crate::new_game_screen::setvars_discovery::{setvars_from, SetvarItem};

/// What the page needs to list the options: the vehicle and its repaints, read once.
#[derive(Debug, Clone, Default)]
pub(super) struct LiveSetvars {
    pub vehicle: Option<VehicleId>,
    pub repaints: Option<VehicleRepaints>,
    /// The active vehicle's changes as the last build showed them (the page follows changes made
    /// elsewhere, e.g. through the control protocol).
    pub shown: Option<Vec<(String, i32)>>,
}

/// The options of the active vehicle with their current values.
pub(super) fn items_of(menu: &VehicleMenu, registry: &SessionVehicles) -> Vec<SetvarItem> {
    let (Some(record), Some(reps)) = (
        registry
            .active()
            .filter(|r| r.is_alive() && menu.live_setvars.vehicle == Some(r.id)),
        menu.live_setvars.repaints.as_ref(),
    ) else {
        // Another vehicle became active since the page was opened: no stale options.
        return Vec::new();
    };
    let custom: HashMap<String, i32> = record.setvars.iter().cloned().collect();
    setvars_from(reps, &custom, &record.repaint)
}

pub(super) fn build_setvars(
    commands: &mut Commands,
    menu: &VehicleMenu,
    registry: &SessionVehicles,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(44.0, 80.0)
        .with_width_range(480.0, 760.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    let items = items_of(menu, registry);
    let name = registry
        .active()
        .map(|r| r.name.clone())
        .unwrap_or_default();
    commands.entity(content).with_children(|p| {
        label(p, "Setvars", 22.0, ColorRole::Primary);
        label(p, name, 13.0, ColorRole::OnSurfaceVariant);
        label(
            p,
            "Changes go to the running vehicle at once. An option its scripts read only at start shows no effect until the vehicle is built again (spawn it from Configurations).",
            12.0,
            ColorRole::OnSurfaceVariant,
        );
        if items.is_empty() {
            label(
                p,
                "There is no vehicle with equipment options to change.",
                13.0,
                ColorRole::OnSurface,
            );
        }
        scroll_list(p, 4, menu.scroll[4], Val::Percent(100.0), |list| {
            setvar_rows(list, &items, true);
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(12.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|footer| {
            let reset = button(footer, "Reset to repaint", false, Action::LiveReset);
            footer
                .commands()
                .entity(reset)
                .insert(UiId::new("vehicle.setvars.reset"));
            button(footer, "Back", true, Action::Back);
        });
    });
}

/// Opens the page: reads the repaints of the active vehicle once (its files).
pub(super) fn open(world: &mut World) {
    let record = world
        .get_resource::<SessionVehicles>()
        .and_then(|r| r.active().filter(|r| r.is_alive()).cloned());
    let repaints = record
        .as_ref()
        .and_then(|r| crate::control_setvars::repaints_of_record(world, r));
    if let Some(mut menu) = world.get_resource_mut::<VehicleMenu>() {
        menu.page = Page::Setvars;
        menu.status.clear();
        menu.live_setvars = LiveSetvars {
            vehicle: record.map(|r| r.id),
            repaints,
            shown: None,
        };
    }
}

/// Applies one change of the page to the active vehicle: `toggle` flips a switch, otherwise the
/// option moves by `delta`.
pub(super) fn change(world: &mut World, name: &str, delta: Option<i32>) {
    let (new, default) = {
        let (Some(menu), Some(registry)) = (
            world.get_resource::<VehicleMenu>(),
            world.get_resource::<SessionVehicles>(),
        ) else {
            return;
        };
        let items = items_of(menu, registry);
        let Some(item) = items.iter().find(|i| i.name == name) else {
            return;
        };
        let next = match delta {
            Some(d) => item.step(item.value, d),
            None => i32::from(item.value == 0),
        };
        (next, item.default_value)
    };
    crate::control_setvars::apply_live(world, name, new, default);
    // The page shows the new value.
    if let Some(mut menu) = world.get_resource_mut::<VehicleMenu>() {
        menu.status.clear();
    }
}

/// Puts every option back to the value its repaint gives it.
pub(super) fn reset(world: &mut World) {
    let changes: Vec<(String, i32)> = {
        let (Some(menu), Some(registry)) = (
            world.get_resource::<VehicleMenu>(),
            world.get_resource::<SessionVehicles>(),
        ) else {
            return;
        };
        items_of(menu, registry)
            .into_iter()
            .filter(|i| i.value != i.default_value)
            .map(|i| (i.name, i.default_value))
            .collect()
    };
    for (name, default) in changes {
        crate::control_setvars::apply_live(world, &name, default, default);
    }
    if let Some(mut menu) = world.get_resource_mut::<VehicleMenu>() {
        menu.status.clear();
    }
}

#[cfg(test)]
mod tests;
