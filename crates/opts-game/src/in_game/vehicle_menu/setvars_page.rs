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
mod tests {
    use super::*;
    use crate::new_game_screen::repaints::VehicleRepaints;
    use crate::session_vehicles::SpawnParams;
    use std::collections::BTreeMap;

    /// Repaint `Red`: `Heater` is a switch (0 or 1), `Seats` a choice of 1 to 3.
    fn repaints() -> VehicleRepaints {
        let mut setvars = BTreeMap::new();
        setvars.insert(
            "Red".to_string(),
            vec![
                ("Heater".to_string(), "0".to_string()),
                ("Seats".to_string(), "1".to_string()),
            ],
        );
        setvars.insert(
            "Blue".to_string(),
            vec![
                ("Heater".to_string(), "1".to_string()),
                ("Seats".to_string(), "3".to_string()),
            ],
        );
        VehicleRepaints {
            default_name: "Red".into(),
            names: vec!["Red".into(), "Blue".into()],
            setvars,
            ..Default::default()
        }
    }

    fn app() -> App {
        let mut app = App::new();
        let mut registry = SessionVehicles::default();
        let id = registry.register_alive(
            SpawnParams {
                catalogue_id: "bus".into(),
                name: "Bus".into(),
                bus_file: "/bus.bus".into(),
                ..Default::default()
            },
            Entity::PLACEHOLDER,
            None,
            None,
            None,
        );
        registry.set_active(Some(id));
        app.insert_resource(registry).insert_resource(VehicleMenu {
            live_setvars: LiveSetvars {
                vehicle: Some(id),
                repaints: Some(repaints()),
                shown: None,
            },
            ..Default::default()
        });
        app
    }

    fn values(app: &App) -> Vec<(String, i32)> {
        let menu = app.world().resource::<VehicleMenu>();
        let registry = app.world().resource::<SessionVehicles>();
        items_of(menu, registry)
            .into_iter()
            .map(|i| (i.name, i.value))
            .collect()
    }

    #[test]
    fn the_page_lists_the_options_with_the_repaints_values() {
        let app = app();
        let v = values(&app);
        assert!(v.contains(&("Heater".to_string(), 0)), "{v:?}");
        assert!(v.contains(&("Seats".to_string(), 1)), "{v:?}");
    }

    #[test]
    fn toggling_and_stepping_change_the_record_and_reset_undoes_them() {
        let mut app = app();
        change(app.world_mut(), "Heater", None);
        change(app.world_mut(), "Seats", Some(1));
        let v = values(&app);
        assert!(v.contains(&("Heater".to_string(), 1)), "{v:?}");
        assert!(v.contains(&("Seats".to_string(), 3)), "{v:?}");
        // Both are kept on the session record, which the save and the configurations read.
        let record = app.world().resource::<SessionVehicles>().active().cloned();
        assert_eq!(record.map(|r| r.setvars.len()), Some(2));

        reset(app.world_mut());
        let v = values(&app);
        assert!(v.contains(&("Heater".to_string(), 0)), "{v:?}");
        assert!(v.contains(&("Seats".to_string(), 1)), "{v:?}");
        let record = app.world().resource::<SessionVehicles>().active().cloned();
        assert_eq!(record.map(|r| r.setvars.len()), Some(0));
    }
}
