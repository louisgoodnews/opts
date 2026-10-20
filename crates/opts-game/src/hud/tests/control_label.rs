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

#[test]
fn the_key_hint_line_shows_the_players_keys() {
    let mut app = app();
    build(&mut app, super::super::widget::VehicleKind::Bus);
    let hint = |app: &mut App| -> String {
        app.world_mut()
            .query::<(&super::super::build::HudText, &Text)>()
            .iter(app.world())
            .find(|(h, _)| h.binding == super::super::widget::Binding::ControlsHint)
            .map(|(_, t)| t.0.clone())
            .unwrap_or_default()
    };
    for _ in 0..3 {
        app.update();
    }
    let before = hint(&mut app);
    assert!(
        before.contains(". Parking brake") && before.contains("F1 Help"),
        "{before}"
    );
    let mut keys = opts_bevy::ActionKeys::default();
    keys.set("parking_brake", opts_bevy::KeyChord::parse("B").unwrap());
    keys.set("help", opts_bevy::KeyChord::parse("Ctrl+H").unwrap());
    app.insert_resource(keys);
    for _ in 0..3 {
        app.update();
    }
    let after = hint(&mut app);
    assert!(
        after.contains("B Parking brake") && after.contains("Ctrl+H Help"),
        "{after}"
    );
}
