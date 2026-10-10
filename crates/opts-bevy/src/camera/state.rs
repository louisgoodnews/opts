//! Camera mode enumeration and the shared [`CameraState`] resource.

use bevy::ecs::system::Resource;
use bevy::math::Vec3;

/// Minimum (and initial) free-roam speed in metres per second.
pub const FREE_ROAM_MIN_SPEED: f32 = 5.0;

/// Speed multiplier applied per wheel notch in free roam (reciprocal for decrease).
pub const FREE_ROAM_SPEED_FACTOR: f32 = 1.25;

/// Camera perspective selected with the F-keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CameraMode {
    /// Key 1 — the player's human character (requires a human; not yet available).
    Ego,
    /// Key 2 — driver's cockpit of the current vehicle.
    Cockpit,
    /// Key 3 — external view of the current vehicle.
    External,
    /// Key 4 — passenger views (not yet available).
    Passenger,
    /// Key 5 — detached free-flying camera.
    FreeRoam,
}

impl CameraMode {
    /// All modes in HUD order.
    pub const ALL: [CameraMode; 5] = [
        CameraMode::Ego,
        CameraMode::Cockpit,
        CameraMode::External,
        CameraMode::Passenger,
        CameraMode::FreeRoam,
    ];

    /// The number key that selects this mode.
    #[must_use]
    pub const fn key_number(self) -> u8 {
        match self {
            CameraMode::Ego => 1,
            CameraMode::Cockpit => 2,
            CameraMode::External => 3,
            CameraMode::Passenger => 4,
            CameraMode::FreeRoam => 5,
        }
    }

    /// Short HUD label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            CameraMode::Ego => "Ego",
            CameraMode::Cockpit => "Cockpit",
            CameraMode::External => "External",
            CameraMode::Passenger => "Pax",
            CameraMode::FreeRoam => "Free",
        }
    }
}

/// Which camera modes can currently be entered. Refreshed every frame from the world.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CameraAvailability {
    /// A player human exists.
    pub ego: bool,
    /// A user vehicle exists.
    pub cockpit: bool,
    /// A user vehicle exists (external view needs a target).
    pub external: bool,
    /// A user vehicle with passenger cameras exists.
    pub passenger: bool,
}

impl CameraAvailability {
    /// Whether `mode` can be entered. Free roam is always available.
    #[must_use]
    pub const fn allows(&self, mode: CameraMode) -> bool {
        match mode {
            CameraMode::Ego => self.ego,
            CameraMode::Cockpit => self.cockpit,
            CameraMode::External => self.external,
            CameraMode::Passenger => self.passenger,
            CameraMode::FreeRoam => true,
        }
    }
}

/// Shared camera state. Free-roam speed persists across mode switches for the whole run.
#[derive(Resource, Debug, Clone)]
pub struct CameraState {
    /// Active mode.
    pub mode: CameraMode,
    /// Mode that was active before the last mode change (target of key 0).
    pub previous_mode: CameraMode,
    /// Index into the active mode's sub-camera list.
    pub sub_camera_index: usize,
    /// Free-roam speed in m/s (never below [`FREE_ROAM_MIN_SPEED`]).
    pub free_roam_speed: f32,
    /// External-view distance in metres.
    pub external_zoom: f32,
    /// External-view yaw in radians.
    pub external_yaw: f32,
    /// External-view pitch in radians.
    pub external_pitch: f32,
    /// External-view orbit-centre offset in metres.
    pub pan_offset: Vec3,
    /// Free-roam position when free roam was last entered (target of the free-roam reset).
    pub free_roam_spawn: Vec3,
    /// Free-roam yaw when free roam was last entered.
    pub free_roam_spawn_yaw: f32,
    /// Free-roam pitch when free roam was last entered.
    pub free_roam_spawn_pitch: f32,
    /// Current free-roam yaw in radians.
    pub free_roam_yaw: f32,
    /// Current free-roam pitch in radians.
    pub free_roam_pitch: f32,
    /// Look yaw offset in cockpit and passenger views (right-drag), radians.
    pub view_yaw: f32,
    /// Look pitch offset in cockpit and passenger views, radians.
    pub view_pitch: f32,
    /// Ego-view look yaw relative to the head, radians.
    pub ego_yaw: f32,
    /// Ego-view look pitch relative to the head, radians.
    pub ego_pitch: f32,
    /// Mouse-look sensitivity in radians per pixel.
    pub mouse_sensitivity: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            mode: CameraMode::FreeRoam,
            previous_mode: CameraMode::FreeRoam,
            sub_camera_index: 0,
            free_roam_speed: FREE_ROAM_MIN_SPEED,
            external_zoom: 8.0,
            external_yaw: 0.0,
            external_pitch: -15.0_f32.to_radians(),
            pan_offset: Vec3::ZERO,
            free_roam_spawn: Vec3::ZERO,
            free_roam_spawn_yaw: 0.0,
            free_roam_spawn_pitch: 0.0,
            free_roam_yaw: 0.0,
            free_roam_pitch: 0.0,
            view_yaw: 0.0,
            view_pitch: 0.0,
            ego_yaw: 0.0,
            ego_pitch: 0.0,
            mouse_sensitivity: 0.002,
        }
    }
}

impl CameraState {
    /// Switches to `mode`, remembering the old mode for key 0. Resets per-mode transient state.
    ///
    /// Does nothing if `mode` is already active.
    pub fn switch_to(&mut self, mode: CameraMode) {
        if mode == self.mode {
            return;
        }
        self.previous_mode = self.mode;
        self.mode = mode;
        self.sub_camera_index = 0;
        self.pan_offset = Vec3::ZERO;
        self.ego_yaw = 0.0;
        self.ego_pitch = 0.0;
        self.view_yaw = 0.0;
        self.view_pitch = 0.0;
    }
}

/// Next sub-camera index after `notches` wheel notches (positive = forward), wrapping around a
/// list of `len` entries. Returns 0 for an empty list.
#[must_use]
pub fn cycle_index(current: usize, notches: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as i64 + i64::from(notches)).rem_euclid(len as i64) as usize
}

/// Applies `notches` wheel notches to a free-roam speed: ×1.25 per up-notch, ×0.8 per
/// down-notch, clamped to the 5 m/s minimum with no upper bound.
#[must_use]
pub fn scale_free_roam_speed(speed: f32, notches: f32) -> f32 {
    (speed * FREE_ROAM_SPEED_FACTOR.powf(notches)).max(FREE_ROAM_MIN_SPEED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_never_drops_below_minimum_and_has_no_cap() {
        assert_eq!(scale_free_roam_speed(5.0, -3.0), 5.0);
        assert!((scale_free_roam_speed(5.0, 1.0) - 6.25).abs() < 1e-5);
        assert!(scale_free_roam_speed(5.0, 100.0) > 1.0e9);
    }

    #[test]
    fn sub_camera_cycling_wraps_both_ways() {
        assert_eq!(cycle_index(2, 1, 3), 0);
        assert_eq!(cycle_index(0, -1, 3), 2);
        assert_eq!(cycle_index(1, 7, 3), 2);
        assert_eq!(cycle_index(5, 1, 0), 0);
    }

    #[test]
    fn up_then_down_is_reciprocal() {
        let s = scale_free_roam_speed(scale_free_roam_speed(22.0, 4.0), -4.0);
        assert!((s - 22.0).abs() < 1e-3);
    }

    #[test]
    fn switch_remembers_previous_and_is_idempotent() {
        let mut s = CameraState::default();
        s.switch_to(CameraMode::Cockpit);
        assert_eq!(
            (s.mode, s.previous_mode),
            (CameraMode::Cockpit, CameraMode::FreeRoam)
        );
        s.switch_to(CameraMode::Cockpit);
        assert_eq!(s.previous_mode, CameraMode::FreeRoam);
    }
}
