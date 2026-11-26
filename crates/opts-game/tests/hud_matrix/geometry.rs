//! Layout geometry of the HUD: rectangles, visibility and overlap.

use bevy::prelude::*;

/// The layout rectangle `[x, y, w, h]` (layout pixels) of the element with `id`.
pub fn rect(app: &mut App, id: &str) -> Option<[f32; 4]> {
    let e = opts_gui::ui_tree::find_entity(app.world_mut(), id)?;
    opts_gui::ui_tree::rect_of(app.world(), e)
}

/// Whether an entity or an ancestor is not shown.
pub fn hidden(app: &App, mut e: Entity) -> bool {
    loop {
        let w = app.world();
        if w.get::<Node>(e).is_some_and(|n| n.display == Display::None)
            || w.get::<Visibility>(e) == Some(&Visibility::Hidden)
        {
            return true;
        }
        match w.get::<ChildOf>(e) {
            Some(p) => e = p.parent(),
            None => return false,
        }
    }
}

/// Whether two rectangles overlap by more than half a pixel.
pub fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] + 0.5 < b[0] + b[2]
        && b[0] + 0.5 < a[0] + a[2]
        && a[1] + 0.5 < b[1] + b[3]
        && b[1] + 0.5 < a[1] + a[3]
}
