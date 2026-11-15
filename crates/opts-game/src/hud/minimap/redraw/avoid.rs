//! The mini map steps aside while the viewpoint editor's panel owns the right edge.

use super::super::*;

/// The viewpoint editor's panel owns the right edge while it is open, so the map steps aside: it
/// is hidden (not removed, so nothing moves) and comes back when the editor closes.
pub fn avoid_viewpoint_panel(
    editor: Option<Res<opts_bevy::ViewpointState>>,
    mut maps: Query<&mut Visibility, With<crate::hud::parts::MapBox>>,
) {
    let want = if editor.is_some_and(|e| e.editing) {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut vis in &mut maps {
        if *vis != want {
            *vis = want;
        }
    }
}
