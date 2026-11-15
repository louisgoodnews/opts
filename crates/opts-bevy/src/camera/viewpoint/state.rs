//! The editor state: what the camera shows, what is stored and the editor rows and fields.

use super::*;

/// The editor and the viewpoint the cockpit camera applies.
#[derive(Resource, Debug, Clone, Default)]
pub struct ViewpointState {
    /// What the camera shows (the draft while editing).
    pub active: Viewpoint,
    /// The stored viewpoint of the current vehicle and camera (what Cancel returns to).
    pub committed: Viewpoint,
    /// The editor is open and owns the keyboard.
    pub editing: bool,
    /// The selected row.
    pub row: ViewpointRow,
    /// The FOV of the vehicle's own preset for the camera in use (set by the camera system).
    pub default_fov_deg: f32,
    /// The FOV the user chose as the default for the driver's views (`camera.default_fov`),
    /// used where the viewpoint has no FOV of its own; `None` keeps the vehicle's.
    pub user_default_fov_deg: Option<f32>,
    /// The vehicle the viewpoint belongs to (its catalogue identity), when known.
    pub vehicle_key: Option<String>,
    /// The vehicle's model (its folder), when known: viewpoints saved with the model scope apply
    /// to every vehicle of it.
    pub model_key: Option<String>,
    /// The editor saves for the whole model instead of this vehicle.
    pub scope_model: bool,
    /// The driver camera index the viewpoint belongs to.
    pub camera_index: usize,
    /// The vehicle and camera `active`/`committed` were loaded for (set by the loader).
    pub loaded_for: Option<(String, usize)>,
    /// A write the application still has to do.
    pub request: Option<ViewpointRequest>,
}

impl ViewpointState {
    /// The FOV the camera uses: the viewpoint's or the vehicle's own.
    #[must_use]
    pub fn effective_fov_deg(&self) -> f32 {
        self.active
            .fov_deg
            .or(self.user_default_fov_deg)
            .unwrap_or(self.default_fov_deg)
    }

    /// Whether the editor holds unsaved changes.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.active != self.committed
    }

    /// Opens the editor on the current viewpoint.
    pub fn open(&mut self) {
        if !self.editing {
            self.editing = true;
            self.scope_model = false;
            self.row = ViewpointRow::X;
        }
    }

    /// Closes the editor and returns to the stored viewpoint.
    pub fn cancel(&mut self) {
        if self.editing {
            self.active = self.committed;
        }
        self.editing = false;
        self.scope_model = false;
    }

    /// Stores the shown viewpoint (when it changed, or when asked for) and closes the editor.
    pub fn save(&mut self) {
        let vp = self.active.clamped();
        self.active = vp;
        let model = self.scope_model && self.model_key.is_some();
        if vp != self.committed || model {
            self.committed = vp;
            self.request = Some(if model {
                ViewpointRequest::SaveModel(vp)
            } else {
                ViewpointRequest::Save(vp)
            });
        }
        self.editing = false;
        self.scope_model = false;
    }

    /// Back to the vehicle-defined view (the draft; *Save* stores it, which removes the entry).
    pub fn reset(&mut self) {
        self.active = Viewpoint::default();
    }

    /// Sets the shown viewpoint (clamped); used by the protocol and the pointer.
    pub fn set(&mut self, vp: Viewpoint) {
        self.active = vp.clamped();
    }

    /// Adjusts one row by `steps` (one step is 1 cm or 1 degree), keeping it inside its limit.
    pub fn adjust(&mut self, row: ViewpointRow, steps: f32) {
        let mut vp = self.active;
        match row {
            ViewpointRow::X => vp.x += steps * OFFSET_STEP_M,
            ViewpointRow::Y => vp.y += steps * OFFSET_STEP_M,
            ViewpointRow::Z => vp.z += steps * OFFSET_STEP_M,
            ViewpointRow::Fov => {
                vp.fov_deg = Some(self.effective_fov_deg() + steps * FOV_STEP_DEG);
            }
        }
        self.active = vp.clamped();
    }

    /// Sets one row to a fraction (0..=1) of its range, as a click on its track does.
    pub fn set_fraction(&mut self, row: ViewpointRow, fraction: f32) {
        let f = fraction.clamp(0.0, 1.0);
        let mut vp = self.active;
        let offset = (f * 2.0 - 1.0) * OFFSET_LIMIT_M;
        match row {
            ViewpointRow::X => vp.x = offset,
            ViewpointRow::Y => vp.y = offset,
            ViewpointRow::Z => vp.z = offset,
            ViewpointRow::Fov => {
                vp.fov_deg = Some(FOV_RANGE_DEG.0 + f * (FOV_RANGE_DEG.1 - FOV_RANGE_DEG.0));
            }
        }
        self.active = vp.clamped();
    }

    /// The value of `row` as a fraction (0..=1) of its range (the slider position).
    #[must_use]
    pub fn fraction(&self, row: ViewpointRow) -> f32 {
        let off = |v: f32| (v / OFFSET_LIMIT_M * 0.5 + 0.5).clamp(0.0, 1.0);
        match row {
            ViewpointRow::X => off(self.active.x),
            ViewpointRow::Y => off(self.active.y),
            ViewpointRow::Z => off(self.active.z),
            ViewpointRow::Fov => ((self.effective_fov_deg() - FOV_RANGE_DEG.0)
                / (FOV_RANGE_DEG.1 - FOV_RANGE_DEG.0))
                .clamp(0.0, 1.0),
        }
    }
}
