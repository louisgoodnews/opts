//! The editor keys: the open and close keys, the arrows that adjust a row and the repeat of held keys.

use super::*;

/// Run condition: the viewpoint editor is not capturing the keyboard (driving and script keys
/// stand still while it is open).
#[must_use]
pub fn not_editing_viewpoint(state: Option<Res<ViewpointState>>) -> bool {
    !state.is_some_and(|s| s.editing)
}

const REPEAT_DELAY: f32 = 0.35;
const REPEAT_INTERVAL: f32 = 0.05;

/// Shift + Tab opens the editor in the driver's view (Tab saves and closes it); while it is open the arrows pick a row
/// and adjust it (Shift: ten steps), Enter saves, Esc cancels, Home resets to the vehicle's view.
///
/// Shift + Tab does nothing in other views, so it never steals a key there. Leaving the driver's view or
/// changing the sub camera cancels an open editor.
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
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if keys.just_pressed(KeyCode::Tab) {
        // Plain Tab belongs to the virtual mirrors (`crate::mirrors`); inside the editor it saves.
        if state.editing {
            state.save();
        } else if shift {
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
