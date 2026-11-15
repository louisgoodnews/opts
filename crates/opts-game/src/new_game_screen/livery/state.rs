//! The livery state of a vehicle: its discovery request, the poll that collects it, and the components.

use super::*;

/// The liveries and options found for one vehicle.
#[derive(Resource, Default)]
pub struct LiveryState {
    /// Vehicle (id) the data belongs to.
    pub vehicle: String,
    /// The repaints, once discovered.
    pub repaints: Option<VehicleRepaints>,
    task: Option<(String, Task<VehicleRepaints>)>,
}

/// Container of the livery buttons in the vehicle detail.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct LiveryBox(pub String);

/// Container of the *Configure…* button (or the no-options line).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct OptionsBox(pub String);

/// The swatch of a livery button; the thumbnail service colours it from the livery's texture.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct LiverySwatch {
    /// Vehicle id.
    pub vehicle: String,
    /// Livery name.
    pub livery: String,
}

/// Starts discovery for `vehicle` unless it is cached or running.
pub fn request(state: &mut LiveryState, vehicle: &VehicleItem) {
    if state.vehicle == vehicle.id && (state.repaints.is_some() || state.task.is_some()) {
        return;
    }
    state.vehicle.clone_from(&vehicle.id);
    state.repaints = None;
    let v = vehicle.clone();
    state.task = Some((
        vehicle.id.clone(),
        IoTaskPool::get().spawn(async move { vehicle_repaints(&v) }),
    ));
}

/// Collects finished discovery and fills the boxes of the vehicle.
pub fn poll_liveries(
    mut state: ResMut<LiveryState>,
    catalogue: Res<GameCatalogue>,
    setup: Res<NewGameSetup>,
    boxes: Query<(Entity, &LiveryBox)>,
    options: Query<(Entity, &OptionsBox)>,
    mut commands: Commands,
) {
    let finished = match state.task.as_mut().and_then(|(_, t)| check_ready(t)) {
        Some(r) => r,
        None => return,
    };
    let Some((id, _)) = state.task.take() else {
        return;
    };
    if id != state.vehicle {
        return;
    }
    state.repaints = Some(finished);
    let Some(vehicle) = catalogue.vehicles.iter().find(|v| v.id == id) else {
        return;
    };
    let reps = state.repaints.clone().unwrap_or_default();
    for (e, b) in &boxes {
        if b.0 == id {
            fill_liveries(&mut commands, e, vehicle, &reps);
        }
    }
    for (e, b) in &options {
        if b.0 == id {
            fill_options(&mut commands, e, &reps, &setup);
        }
    }
}
