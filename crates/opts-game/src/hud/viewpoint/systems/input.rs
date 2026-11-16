//! The panel's buttons and tracks: a button steps, resets, cancels, saves or switches the scope, and a
//! click or drag on a track sets the row under the cursor.

use super::*;

/// The buttons of the panel.
pub fn panel_buttons(
    mut clicks: MessageReader<ButtonClicked>,
    buttons: Query<&ViewpointButton>,
    mut state: ResMut<ViewpointState>,
) {
    for ButtonClicked(entity) in clicks.read() {
        let Ok(action) = buttons.get(*entity) else {
            continue;
        };
        if !state.editing {
            continue;
        }
        match *action {
            ViewpointButton::Step(row, dir) => {
                state.row = row;
                state.adjust(row, f32::from(dir));
            }
            ViewpointButton::Reset => state.reset(),
            ViewpointButton::Cancel => state.cancel(),
            ViewpointButton::Save => state.save(),
            ViewpointButton::Scope => {
                if state.model_key.is_some() {
                    state.scope_model = !state.scope_model;
                }
            }
        }
    }
}

/// Click or drag on a track sets the row to the position under the cursor.
pub fn panel_tracks(
    tracks: Query<(&ViewpointTrack, &RelativeCursorPosition), With<Pressed>>,
    mut state: ResMut<ViewpointState>,
) {
    if !state.editing {
        return;
    }
    for (track, cursor) in &tracks {
        if let Some(pos) = cursor.normalized {
            state.row = track.0;
            state.set_fraction(track.0, pos.x + 0.5);
        }
    }
}
