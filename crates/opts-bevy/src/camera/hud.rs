//! Camera hint line shown in the HUD.

use crate::camera::state::{CameraAvailability, CameraMode, CameraState};

/// Builds the one-line camera hint.
///
/// Every mode is listed; the active one is wrapped in `*…*` and unavailable ones in `(…)`
/// (the HUD renders these with highlight/dim styling). Free roam appends its speed, and the
/// `[Shift+Space] Reset` hint is always shown because a mode is always active.
#[must_use]
pub fn camera_hint_text(
    state: &CameraState,
    avail: &CameraAvailability,
    sub_total: usize,
) -> String {
    let mut parts = vec!["[0] Prev".to_string()];
    for mode in CameraMode::ALL {
        let mut label = format!("[{}] {}", mode.key_number(), mode.label());
        if mode == CameraMode::FreeRoam && state.mode == mode {
            label.push_str(&format!("  {:.1} m/s", state.free_roam_speed));
        }
        if mode == state.mode {
            if matches!(mode, CameraMode::Cockpit | CameraMode::Passenger) && sub_total > 0 {
                label.push_str(&format!(" [{}/{}]", state.sub_camera_index + 1, sub_total));
            }
            label = format!("*{label}*");
        } else if !avail.allows(mode) {
            label = format!("({label})");
        }
        parts.push(label);
    }
    let cursor = if matches!(state.mode, CameraMode::FreeRoam | CameraMode::Ego) {
        "   [Alt] Cursor"
    } else {
        ""
    };
    format!("{}   [Shift+Space] Reset{cursor}", parts.join("  "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_shows_sub_camera_index() {
        let s = CameraState {
            mode: CameraMode::Passenger,
            sub_camera_index: 1,
            ..CameraState::default()
        };
        let avail = CameraAvailability {
            passenger: true,
            ..CameraAvailability::default()
        };
        assert!(camera_hint_text(&s, &avail, 7).contains("*[4] Pax [2/7]*"));
    }

    #[test]
    fn hint_marks_active_unavailable_and_speed() {
        let s = CameraState {
            free_roam_speed: 22.5,
            ..CameraState::default()
        };
        let t = camera_hint_text(&s, &CameraAvailability::default(), 0);
        assert!(t.contains("*[5] Free  22.5 m/s*"));
        assert!(t.contains("([1] Ego)") && t.contains("([2] Cockpit)"));
        assert!(t.contains("[Shift+Space] Reset"));
    }
}
