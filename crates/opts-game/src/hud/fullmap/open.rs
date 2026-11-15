//! Opening and closing the full-size map, and following the active vehicle.

use super::*;

/// The active vehicle's (else the camera's) position and heading.
pub(super) fn active_pose(
    vehicles: &Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: &Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) -> Option<(DVec2, f32)> {
    vehicles
        .iter()
        .find(|(_, _, player)| *player)
        .map(|(t, _, _)| pose_of(t))
        .or_else(|| cameras.iter().next().map(pose_of))
}

/// F6 opens and closes the map, Esc closes it, a protocol request does either. Opening needs the
/// session to be loaded and no other dialog (report, vehicle menu, pause) to hold the input.
pub fn toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    gates: params::MapGates,
    poses: params::MapPoses,
    mut overview: ResMut<OverviewRoads>,
    mut state: ResMut<FullMapState>,
) {
    let params::MapGates {
        load,
        report,
        menu,
        job,
    } = gates;
    let params::MapPoses { vehicles, cameras } = poses;
    let request = state.request.take();
    let toggle_key = opts_bevy::action_keys_of(&action_keys).just_pressed(&keyboard, "full_map");
    if state.open {
        if toggle_key
            || keyboard.just_pressed(KeyCode::Escape)
            || request == Some(FullMapRequest::Close)
        {
            state.close();
        }
        return;
    }
    if state.grace > 0 {
        state.grace -= 1;
    }
    let wants_open = toggle_key || request == Some(FullMapRequest::Open);
    // The flag of the menu also carries this map's own hold (just after closing); anything else
    // that holds it keeps the map shut.
    let other_modal =
        report.is_some_and(|m| m.0) || (menu.is_some_and(|m| m.0) && state.grace == 0);
    let loaded = load.is_none_or(|l| l.is_complete);
    if !wants_open || other_modal || !loaded {
        return;
    }
    if state.preset_view {
        state.preset_view = false;
    } else {
        let centre = active_pose(&vehicles, &cameras).map_or(state.viewport.centre, |(p, _)| p);
        state.viewport = Viewport::new(centre, state.remembered_span_m);
        state.follow = true;
    }
    state.open = true;
    state.immediate = true;
    state.drawn = None;
    // The first time the map is opened, a reader starts on the roads of the whole map.
    if let Some(job) = job.filter(|_| !overview.started()) {
        let (root, map_dir, tiles) = job.tile_listing();
        overview.start(root, map_dir, tiles);
    }
}

/// While following, the centre is the active vehicle (else the camera).
pub fn follow(
    vehicles: Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut state: ResMut<FullMapState>,
) {
    if !state.follow {
        return;
    }
    if let Some((pos, _)) = active_pose(&vehicles, &cameras) {
        if state.viewport.centre != pos {
            state.viewport.centre = pos;
        }
    }
}
