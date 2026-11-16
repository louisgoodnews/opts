use super::*;

/// The rectangles of the HUD elements and toasts that are shown, by id.
pub(super) fn shown_rects(app: &mut App) -> Vec<(String, [f32; 4])> {
    let mut shown: Vec<(String, [f32; 4])> = Vec::new();
    let mut ids: Vec<String> = ELEMENTS.iter().map(|s| (*s).to_owned()).collect();
    ids.push("hud.viewpoint".to_owned());
    for id in &ids {
        let Some(e) = opts_gui::ui_tree::find_entity(app.world_mut(), id) else {
            continue;
        };
        if hidden(app, e) {
            continue;
        }
        let Some(r) = rect(app, id) else {
            continue;
        };
        shown.push((id.clone(), r));
    }
    // Toasts have no id.
    let toasts: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<opts_gui::components::toast::M3Toast>>()
        .iter(app.world())
        .collect();
    for (i, t) in toasts.iter().enumerate() {
        if hidden(app, *t) {
            continue;
        }
        if let Some(r) = opts_gui::ui_tree::rect_of(app.world(), *t) {
            shown.push((format!("toast{i}"), r));
        }
    }
    shown
}

/// Findings about the shown rectangles: missing, outside the window or overlapping.
pub(super) fn layout_findings(
    what: &str,
    shown: &[(String, [f32; 4])],
    logical: Vec2,
    extras: Extras,
) -> Vec<String> {
    let mut found = Vec::new();
    for needed in ["hud.slot.cluster", "hud.minimap", "hud.pause"] {
        if !shown.iter().any(|(id, _)| id == needed) && !(extras.editor && needed == "hud.minimap")
        {
            found.push(format!("{what}: {needed} is not shown"));
        }
    }
    for (id, r) in shown {
        if r[0] < -0.5
            || r[1] < -0.5
            || r[0] + r[2] > logical.x + 0.5
            || r[1] + r[3] > logical.y + 0.5
        {
            found.push(format!("{what}: {id} {r:?} leaves the window {logical:?}"));
        }
    }
    for (i, (a_id, a)) in shown.iter().enumerate() {
        for (b_id, b) in &shown[i + 1..] {
            if a_id.starts_with("toast") && b_id.starts_with("toast") {
                continue;
            }
            // A layout area this small has no room for a toast beside the top bar and the label:
            // the toast (4 s) may cover them (documented in the HUD registry).
            let toast_pair = a_id.starts_with("toast") || b_id.starts_with("toast");
            if toast_pair && (logical.x < 640.0 || logical.y < 400.0) {
                let other = if a_id.starts_with("toast") {
                    b_id
                } else {
                    a_id
                };
                if other != "hud.minimap" && other != "hud.slot.cluster" {
                    continue;
                }
            }
            if overlap(*a, *b) {
                found.push(format!("{what}: {a_id} {a:?} overlaps {b_id} {b:?}"));
            }
        }
    }
    found
}
