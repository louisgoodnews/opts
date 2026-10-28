//! Bevy resource, systems, and plugin for loading and persisting driver viewpoints.

use super::*;

/// The store and where it lives (`None` when no user data directory is known).
#[derive(Resource, Debug, Clone, Default)]
pub struct Viewpoints {
    /// The file; tests point it at a temporary directory.
    pub path: Option<PathBuf>,
    /// What the file holds (as last read or written).
    pub store: ViewpointStore,
}

impl Viewpoints {
    /// Reads the default file.
    #[must_use]
    pub fn load() -> Self {
        let path = ViewpointStore::path();
        let store = path
            .as_deref()
            .map(ViewpointStore::load_from)
            .unwrap_or_default();
        Self { path, store }
    }

    /// Re-reads the file.
    pub fn reload(&mut self) {
        if let Some(p) = &self.path {
            self.store = ViewpointStore::load_from(p);
        }
    }

    /// Stores a viewpoint (for the vehicle, or for its whole model) and writes the file.
    ///
    /// # Errors
    /// The I/O error when the file cannot be written; the in-memory store is updated anyway.
    pub fn save(
        &mut self,
        key: &str,
        model: Option<&str>,
        index: usize,
        vp: Viewpoint,
        whole_model: bool,
    ) -> std::io::Result<()> {
        // Another run may have written other vehicles meanwhile: merge into the file's content.
        let mut store = self
            .path
            .as_deref()
            .map_or_else(|| self.store.clone(), ViewpointStore::load_from);
        match (whole_model, model) {
            (true, Some(m)) => store.set_model(key, m, index, vp),
            _ => store.set_vehicle(key, model, index, vp),
        }
        self.store = store;
        match &self.path {
            Some(p) => self.store.save_to(p),
            None => Ok(()),
        }
    }
}

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
