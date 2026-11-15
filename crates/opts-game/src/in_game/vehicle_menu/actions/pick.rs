//! Choosing the vehicle, repaint, entry or stop, and spawning the chosen vehicle.

use super::*;

/// Choosing the vehicle, repaint, entry or stop, the options, and the spawn itself. Returns the action when this group does not handle it.
pub(super) fn pick_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::PickVehicle(i) => {
            if c.menu.vehicle != Some(i) {
                c.menu.vehicle = Some(i);
                c.menu.repaint.clear();
                c.menu.setvars.clear();
                c.menu.repaints = None;
                c.menu.scroll[1] = 0.0;
                c.menu.scroll[3] = 0.0;
            }
            None
        }
        Action::PickRepaint(name) => {
            c.menu.repaint = name;
            None
        }
        Action::PickEntry(id) => {
            (c.menu.entry, c.menu.stop) = (Some(id), None);
            None
        }
        Action::PickStop(id) => {
            (c.menu.entry, c.menu.stop) = (None, Some(id));
            None
        }
        Action::Toggle(var) => {
            let current = menu_setvar_value(c.menu, c.catalogue, &var);
            c.menu.setvars.insert(var, i32::from(current == 0));
            None
        }
        Action::Step(var, delta) => {
            let veh = c
                .menu
                .vehicle
                .and_then(|i| c.catalogue.and_then(|c| c.vehicles.get(i)))?;
            let items = get_vehicle_setvars(veh, &c.menu.setvars, &c.menu.repaint);
            if let Some(item) = items.iter().find(|s| s.name == var) {
                let next = item.step(item.value, delta);
                c.menu.setvars.insert(var, next);
            }
            None
        }
        Action::Confirm => {
            confirm_spawn(c);
            None
        }
        other => Some(other),
    }
}

/// Spawns the chosen vehicle at the chosen entry or stop, or says what is missing.
fn confirm_spawn(c: &mut MenuCtx) {
    let (Some(cat), Some(job)) = (c.catalogue, c.job) else {
        return;
    };
    let Some(veh) = c.menu.vehicle.and_then(|i| cat.vehicles.get(i)) else {
        c.menu.status = "Choose a vehicle.".into();
        return;
    };
    let entry = match (c.menu.stop, c.menu.entry) {
        (Some(stop), _) => EntrySel::Stop(stop),
        (None, Some(entry)) => EntrySel::Id(entry),
        (None, None) => {
            c.menu.status = "Choose an entry point or a bus stop.".into();
            return;
        }
    };
    let Some(bus) = job.bus_file(&veh.bus_file) else {
        tracing::error!(at = %opts_bevy::position_tag(), bus = %veh.bus_file, "vehicle .bus not found; nothing spawned");
        c.menu.status = format!("ERROR: {} not found in the installation.", veh.bus_file);
        return;
    };
    let mut setvars: Vec<(String, i32)> = c
        .menu
        .setvars
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    setvars.sort();
    c.registry.request_spawn(SpawnParams {
        catalogue_id: veh.id.clone(),
        name: veh.full_name.clone(),
        bus_file: bus.display().to_string(),
        repaint: c.menu.repaint.clone(),
        setvars,
        entry,
        formation: None,
    });
    c.menu.current = veh.full_name.clone();
    c.menu.page = Page::Closed;
    c.menu.config_open = false;
}
