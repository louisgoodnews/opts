//! The viewpoint editor of the driver's view (key 2): the eye position and field of view of the
//! cockpit camera, adjustable with live preview (M-0084).
//!
//! A [`Viewpoint`] is an **offset** from the vehicle's own `[add_camera_driver]` preset, so the
//! default (all zero, no FOV) is exactly the vehicle-defined view. [`ViewpointState`] holds what
//! the camera shows (`active`), what is stored (`committed`) and the editor's state; the keys live
//! in [`handle_viewpoint_keys`] (Tab opens and closes, arrows adjust, Enter saves, Esc cancels,
//! Home resets, M switches between this vehicle and all of its model). Persistence is the
//! application's job: it loads into `committed`/`active` when the
//! vehicle or the sub camera changes and drains [`ViewpointState::request`] to write the file
//! (`opts-game`, `viewpoints.rs`).
//!
//! Axes are the driver's: **X** to the right, **Y** up, **Z** forward, metres.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::time::Real;
use serde::{Deserialize, Serialize};

use crate::camera::state::{CameraAvailability, CameraMode, CameraState};

/// Largest offset on each axis in metres (the eye stays inside the cab).
pub const OFFSET_LIMIT_M: f32 = 0.6;
/// Smallest and largest horizontal field of view in degrees.
pub const FOV_RANGE_DEG: (f32, f32) = (30.0, 120.0);
/// One step of an offset in metres (1 cm).
pub const OFFSET_STEP_M: f32 = 0.01;
/// One step of the field of view in degrees.
pub const FOV_STEP_DEG: f32 = 1.0;
/// The coarse multiplier while Shift is held.
pub const COARSE_FACTOR: f32 = 10.0;

/// A cockpit viewpoint: the offset from the vehicle-defined eye position and an optional FOV.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Viewpoint {
    /// Lateral offset in metres, positive to the right.
    #[serde(default)]
    pub x: f32,
    /// Height offset in metres, positive up.
    #[serde(default)]
    pub y: f32,
    /// Longitudinal offset in metres, positive forward.
    #[serde(default)]
    pub z: f32,
    /// Horizontal field of view in degrees; `None` keeps the vehicle's own.
    #[serde(default)]
    pub fov_deg: Option<f32>,
}

impl Viewpoint {
    /// Whether this is the vehicle-defined view (nothing to store).
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Whether every value is finite and inside its limit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        [self.x, self.y, self.z]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= OFFSET_LIMIT_M + 1e-4)
            && self
                .fov_deg
                .is_none_or(|f| f.is_finite() && (FOV_RANGE_DEG.0..=FOV_RANGE_DEG.1).contains(&f))
    }

    /// The same viewpoint with every value brought inside its limit (non-finite values become 0
    /// or the vehicle's FOV) and rounded to whole centimetres and tenths of a degree.
    #[must_use]
    pub fn clamped(self) -> Self {
        let off = |v: f32| {
            if v.is_finite() {
                ((v.clamp(-OFFSET_LIMIT_M, OFFSET_LIMIT_M)) * 100.0).round() / 100.0
            } else {
                0.0
            }
        };
        Self {
            x: off(self.x),
            y: off(self.y),
            z: off(self.z),
            fov_deg: self
                .fov_deg
                .filter(|f| f.is_finite())
                .map(|f| (f.clamp(FOV_RANGE_DEG.0, FOV_RANGE_DEG.1) * 10.0).round() / 10.0),
        }
    }

    /// The offset in the vehicle's Bevy frame (right = +X, up = +Y, forward = -Z).
    #[must_use]
    pub fn local_offset(&self) -> Vec3 {
        Vec3::new(self.x, self.y, -self.z)
    }
}

/// A row of the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewpointRow {
    /// Lateral offset.
    #[default]
    X,
    /// Height.
    Y,
    /// Longitudinal offset.
    Z,
    /// Field of view.
    Fov,
}

impl ViewpointRow {
    /// All rows in display order.
    pub const ALL: [Self; 4] = [Self::X, Self::Y, Self::Z, Self::Fov];

    /// A stable id (protocol, UI ids).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
            Self::Fov => "fov",
        }
    }

    /// The row after this one (wraps).
    #[must_use]
    pub fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }

    /// The row before this one (wraps).
    #[must_use]
    pub fn prev(self) -> Self {
        Self::ALL[(self as usize + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// What the application has to do with the stored viewpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewpointRequest {
    /// Store this viewpoint for the current vehicle and camera (the default removes the entry,
    /// or keeps an explicit default when the vehicle's model has a viewpoint).
    Save(Viewpoint),
    /// Store this viewpoint for every vehicle of the current model (and drop the current
    /// vehicle's own override).
    SaveModel(Viewpoint),
}

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

/// Run condition: the viewpoint editor is not capturing the keyboard (driving and script keys
/// stand still while it is open).
#[must_use]
pub fn not_editing_viewpoint(state: Option<Res<ViewpointState>>) -> bool {
    !state.is_some_and(|s| s.editing)
}

const REPEAT_DELAY: f32 = 0.35;
const REPEAT_INTERVAL: f32 = 0.05;

/// Tab opens and closes the editor in the driver's view; while it is open the arrows pick a row
/// and adjust it (Shift: ten steps), Enter saves, Esc cancels, Home resets to the vehicle's view.
///
/// Tab does nothing in other views, so it never steals a key there. Leaving the driver's view or
/// changing the sub camera cancels an open editor.
#[allow(clippy::too_many_arguments)]
pub fn handle_viewpoint_keys(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    camera: Res<CameraState>,
    avail: Res<CameraAvailability>,
    mut state: ResMut<ViewpointState>,
    mut held: Local<(Option<KeyCode>, f32)>,
) {
    let in_cockpit = camera.mode == CameraMode::Cockpit && avail.cockpit;
    if state.editing && (!in_cockpit || camera.sub_camera_index != state.camera_index) {
        state.cancel();
        return;
    }
    if !in_cockpit {
        return;
    }
    if keys.just_pressed(KeyCode::Tab) {
        if state.editing {
            state.save();
        } else {
            state.camera_index = camera.sub_camera_index;
            state.open();
        }
        return;
    }
    if !state.editing {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.cancel();
        return;
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        state.save();
        return;
    }
    if keys.just_pressed(KeyCode::Home) {
        state.reset();
    }
    if keys.just_pressed(KeyCode::KeyM) && state.model_key.is_some() {
        state.scope_model = !state.scope_model;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        state.row = state.row.next();
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        state.row = state.row.prev();
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let factor = if shift { COARSE_FACTOR } else { 1.0 };
    for (key, dir) in [(KeyCode::ArrowLeft, -1.0), (KeyCode::ArrowRight, 1.0)] {
        if keys.just_pressed(key) {
            *held = (Some(key), 0.0);
            let row = state.row;
            state.adjust(row, dir * factor);
        } else if keys.pressed(key) && held.0 == Some(key) {
            held.1 += time.delta_secs();
            if held.1 >= REPEAT_DELAY {
                held.1 -= REPEAT_INTERVAL;
                let row = state.row;
                state.adjust(row, dir * factor);
            }
        }
    }
    if held.0.is_some_and(|k| !keys.pressed(k)) {
        *held = (None, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ViewpointState {
        ViewpointState {
            default_fov_deg: 70.0,
            ..default()
        }
    }

    #[test]
    fn default_is_the_vehicle_view() {
        let vp = Viewpoint::default();
        assert!(vp.is_default() && vp.is_valid());
        assert_eq!(vp.local_offset(), Vec3::ZERO);
    }

    #[test]
    fn limits_clamp_and_non_finite_values_vanish() {
        let vp = Viewpoint {
            x: 5.0,
            y: -5.0,
            z: f32::NAN,
            fov_deg: Some(500.0),
        }
        .clamped();
        assert_eq!((vp.x, vp.y, vp.z), (0.6, -0.6, 0.0));
        assert_eq!(vp.fov_deg, Some(120.0));
        assert!(vp.is_valid());
        assert!(!Viewpoint {
            fov_deg: Some(10.0),
            ..default()
        }
        .is_valid());
        assert_eq!(
            Viewpoint {
                fov_deg: Some(10.0),
                ..default()
            }
            .clamped()
            .fov_deg,
            Some(30.0)
        );
    }

    #[test]
    fn forward_offset_points_down_the_bevy_forward_axis() {
        let vp = Viewpoint {
            x: 0.1,
            y: 0.2,
            z: 0.3,
            fov_deg: None,
        };
        assert_eq!(vp.local_offset(), Vec3::new(0.1, 0.2, -0.3));
    }

    #[test]
    fn adjusting_stays_inside_the_limits() {
        let mut s = state();
        s.adjust(ViewpointRow::Y, 1000.0);
        assert_eq!(s.active.y, 0.6);
        s.adjust(ViewpointRow::Fov, 5.0);
        assert_eq!(s.active.fov_deg, Some(75.0));
        s.adjust(ViewpointRow::Fov, -1000.0);
        assert_eq!(s.active.fov_deg, Some(30.0));
    }

    #[test]
    fn save_requests_a_write_once_and_cancel_restores() {
        let mut s = state();
        s.open();
        s.adjust(ViewpointRow::X, 3.0);
        assert!(s.is_dirty());
        s.cancel();
        assert!(!s.editing && s.active == Viewpoint::default());

        s.open();
        s.adjust(ViewpointRow::X, 3.0);
        s.save();
        assert!(!s.editing);
        assert_eq!(
            s.request.take(),
            Some(ViewpointRequest::Save(Viewpoint {
                x: 0.03,
                ..default()
            }))
        );
        s.open();
        s.save();
        assert_eq!(s.request, None, "an unchanged viewpoint writes nothing");
    }

    #[test]
    fn reset_then_save_stores_the_default() {
        let mut s = state();
        s.committed = Viewpoint {
            z: 0.2,
            ..default()
        };
        s.active = s.committed;
        s.open();
        s.reset();
        s.save();
        assert_eq!(
            s.request,
            Some(ViewpointRequest::Save(Viewpoint::default()))
        );
    }

    #[test]
    fn fraction_round_trips() {
        let mut s = state();
        s.set_fraction(ViewpointRow::Fov, 0.5);
        assert_eq!(s.active.fov_deg, Some(75.0));
        assert!((s.fraction(ViewpointRow::Fov) - 0.5).abs() < 1e-3);
        s.set_fraction(ViewpointRow::X, 1.0);
        assert_eq!(s.active.x, 0.6);
        assert!((s.fraction(ViewpointRow::X) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn rows_wrap() {
        assert_eq!(ViewpointRow::Fov.next(), ViewpointRow::X);
        assert_eq!(ViewpointRow::X.prev(), ViewpointRow::Fov);
    }
}
