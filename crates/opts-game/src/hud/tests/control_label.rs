//! The hovered-control label (M-0141).

use super::*;
use crate::hud::control_label::{ControlLabel, ControlLabelText};

fn shown(app: &mut App) -> Option<String> {
    app.world_mut()
        .query_filtered::<&Text, With<ControlLabelText>>()
        .iter(app.world())
        .next()
        .map(|t| t.0.clone())
}

#[test]
fn the_label_follows_the_control_under_the_cursor() {
    let mut app = app();
    app.update();
    assert_eq!(shown(&mut app), None);

    app.world_mut()
        .resource_mut::<opts_bevy::HoveredControl>()
        .0 = Some("cp_kluczyk_mov".into());
    app.update();
    assert_eq!(shown(&mut app).as_deref(), Some("cp_kluczyk_mov"));

    app.world_mut()
        .resource_mut::<opts_bevy::HoveredControl>()
        .0 = Some("horn".into());
    app.update();
    assert_eq!(shown(&mut app).as_deref(), Some("horn"));
    let rows = app
        .world_mut()
        .query_filtered::<(), With<ControlLabel>>()
        .iter(app.world())
        .count();
    assert_eq!(rows, 1, "one label, retexted, not respawned");

    app.world_mut()
        .resource_mut::<opts_bevy::HoveredControl>()
        .0 = None;
    app.update();
    assert_eq!(shown(&mut app), None);
}
