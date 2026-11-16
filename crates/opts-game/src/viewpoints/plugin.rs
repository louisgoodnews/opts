//! Bevy systems and the plugin for loading and persisting driver viewpoints.

use super::*;

/// Loads the stored viewpoint into the state when the vehicle or the camera changes and when the
/// driver's view is entered. An open editor is never touched.
pub fn load_viewpoint(
    camera: Res<CameraState>,
    vehicle: Query<(&ViewpointKey, Option<&ViewpointModel>), With<PlayerVehicle>>,
    mut viewpoints: ResMut<Viewpoints>,
    mut state: ResMut<ViewpointState>,
    mut was_cockpit: Local<bool>,
) {
    let found = vehicle.single().ok();
    let key = found.map(|(k, _)| k.0.clone());
    let model = found.and_then(|(_, m)| m.map(|m| m.0.clone()));
    if state.model_key != model {
        state.model_key.clone_from(&model);
    }
    let entered = camera.mode == CameraMode::Cockpit && !*was_cockpit;
    *was_cockpit = camera.mode == CameraMode::Cockpit;
    if state.vehicle_key != key {
        state.vehicle_key.clone_from(&key);
    }
    if state.editing {
        return;
    }
    let target = key.map(|k| (k, camera.sub_camera_index));
    if state.loaded_for == target && !entered {
        return;
    }
    if entered {
        viewpoints.reload();
    }
    let vp = target
        .as_ref()
        .and_then(|(k, i)| viewpoints.store.lookup(k, model.as_deref(), *i))
        .unwrap_or_default();
    if let Some((k, i)) = &target {
        tracing::debug!(vehicle = %k, camera = i, viewpoint = ?vp, "viewpoint loaded");
        state.camera_index = *i;
    }
    state.active = vp;
    state.committed = vp;
    state.loaded_for = target;
}

/// Hands the *Driver's view: field of view* setting to the viewpoint state (0 or below 30 keeps
/// each vehicle's own field of view).
pub fn sync_default_fov(
    registry: Res<crate::settings_registry::SettingsRegistry>,
    settings: Res<crate::settings::GameSettings>,
    mut state: ResMut<ViewpointState>,
) {
    let fov = registry
        .text(&settings, "graphics.default_fov")
        .and_then(|t| t.parse::<f32>().ok())
        .filter(|f| *f >= opts_bevy::camera::viewpoint::FOV_RANGE_DEG.0)
        .map(|f| f.min(opts_bevy::camera::viewpoint::FOV_RANGE_DEG.1));
    if state.user_default_fov_deg != fov {
        state.user_default_fov_deg = fov;
    }
}

/// Writes what the editor saved (an exclusive system, so the protocol can run it on the spot).
pub fn persist_viewpoint(world: &mut World) {
    let (vp, whole_model) = match world.resource::<ViewpointState>().request {
        Some(ViewpointRequest::Save(vp)) => (vp, false),
        Some(ViewpointRequest::SaveModel(vp)) => (vp, true),
        None => return,
    };
    let (key, model, index) = {
        let mut state = world.resource_mut::<ViewpointState>();
        state.request = None;
        (
            state.vehicle_key.clone(),
            state.model_key.clone(),
            state.camera_index,
        )
    };
    let Some(key) = key else {
        return;
    };
    let result =
        world
            .resource_mut::<Viewpoints>()
            .save(&key, model.as_deref(), index, vp, whole_model);
    let (severity, text) = match result {
        Ok(()) => {
            tracing::info!(vehicle = %key, camera = index, whole_model, viewpoint = ?vp, "viewpoint saved");
            (AlertSeverity::Info, hud_keys::ALERT_VIEWPOINT_SAVED)
        }
        Err(e) => {
            tracing::warn!(vehicle = %key, "cannot write the viewpoints file: {e}");
            (AlertSeverity::Warning, hud_keys::ALERT_VIEWPOINT_FAILED)
        }
    };
    world.write_message(RaiseAlert {
        id: "viewpoint.saved",
        severity,
        key: text,
        args: Vec::new(),
    });
}

/// Registers the store and its systems.
pub struct ViewpointPlugin;

impl Plugin for ViewpointPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Viewpoints>() {
            app.insert_resource(Viewpoints::load());
        }
        app.add_systems(
            Update,
            (
                sync_default_fov
                    .run_if(resource_exists::<crate::settings_registry::SettingsRegistry>)
                    .run_if(resource_exists::<crate::settings::GameSettings>),
                load_viewpoint.after(opts_bevy::camera::input::handle_camera_key_input),
                persist_viewpoint.after(opts_bevy::camera::viewpoint::handle_viewpoint_keys),
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}
