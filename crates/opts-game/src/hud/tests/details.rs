//! The details toast, the place label and the instruments.

use super::*;

#[test]
fn details_toast_dialog_copies_and_reports() {
    let mut app = app();
    app.insert_non_send(opts_gui::components::text_area::TextClipboard::in_memory());
    app.world_mut().write_message(ShowDetailsToast {
        severity: ToastSeverity::Warning,
        message: "Two files failed".into(),
        details: "a.png\nb.png".into(),
    });
    app.update();
    app.update();
    let action = app
        .world_mut()
        .query::<(Entity, &ToastActionButton)>()
        .iter(app.world())
        .map(|(e, _)| e)
        .next()
        .expect("the toast has a Details action");
    assert!(
        app.world_mut()
            .query::<&M3Toast>()
            .iter(app.world())
            .count()
            >= 1
    );
    app.world_mut().write_message(Activated(action));
    app.update();
    app.update();
    let dialog_button = |app: &mut App, choice: &str| {
        app.world_mut()
            .query::<(Entity, &crate::shell::dialog::DialogButton)>()
            .iter(app.world())
            .find(|(_, b)| b.choice == choice)
            .map(|(e, _)| e)
    };
    let copy = dialog_button(&mut app, "copy").expect("Copy button");
    assert!(dialog_button(&mut app, "report").is_some());
    app.world_mut().write_message(Activated(copy));
    app.update();
    app.update();
    assert!(
        dialog_button(&mut app, "copy").is_some(),
        "Copy keeps the dialog open"
    );
    let copied = app
        .world_mut()
        .non_send_mut::<opts_gui::components::text_area::TextClipboard>()
        .get();
    assert_eq!(copied, "Two files failed\n\na.png\nb.png");
    let report = dialog_button(&mut app, "report").expect("Report button");
    app.world_mut().write_message(Activated(report));
    app.update();
    app.update();
    assert!(dialog_button(&mut app, "report").is_none(), "dialog closed");
    let state = *app
        .world()
        .resource::<State<opts_bevy::reporting::ReportModalState>>()
        .get();
    let pending = !matches!(
        app.world()
            .resource::<NextState<opts_bevy::reporting::ReportModalState>>(),
        NextState::Unchanged
    );
    assert!(
        pending || state != opts_bevy::reporting::ReportModalState::Inactive,
        "the report dialog was requested"
    );
}

#[test]
fn the_vehicle_kind_follows_the_components() {
    assert_eq!(values::kind_of(true, false), VehicleKind::Tram);
    assert_eq!(values::kind_of(false, true), VehicleKind::ArticulatedBus);
    assert_eq!(values::kind_of(false, false), VehicleKind::Bus);
}

fn session_text(app: &mut App) -> String {
    app.world_mut()
        .query::<(&HudText, &Text)>()
        .iter(app.world())
        .find(|(h, _)| h.binding == Binding::SessionInfo)
        .map(|(_, t)| t.0.clone())
        .expect("the session widget")
}

#[test]
fn the_place_label_shows_the_spawn_entry_and_only_while_there_is_one() {
    use crate::session_vehicles::PlaceLabel;
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut().resource_mut::<InGameSessionInfo>().map_name = "Testmap".into();
    app.update();
    assert_eq!(session_text(&mut app), "Testmap", "no label, only the map");
    app.world_mut().resource_mut::<PlaceLabel>().0 = Some("Hauptbahnhof".into());
    app.update();
    let text = session_text(&mut app);
    assert!(
        text.contains("Hauptbahnhof") && text.contains("Testmap"),
        "{text}"
    );
    app.world_mut().resource_mut::<PlaceLabel>().0 = None;
    app.update();
    assert_eq!(session_text(&mut app), "Testmap", "the label is gone again");
}

#[test]
fn without_a_vehicle_the_instruments_are_hidden_and_come_back_with_one() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    assert_eq!(display_of(&mut app, "speed"), Some(Display::Flex));
    let players: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::PlayerVehicle>>()
        .iter(app.world())
        .collect();
    for e in players {
        app.world_mut().entity_mut(e).despawn();
    }
    app.update();
    app.update();
    for id in ["speed", "gear", "parking-brake"] {
        assert_eq!(
            display_of(&mut app, id),
            Some(Display::None),
            "{id} without a vehicle"
        );
    }
    assert_eq!(
        display_of(&mut app, "clock"),
        Some(Display::Flex),
        "the status bar stays"
    );
    app.world_mut().spawn((
        opts_bevy::VehicleController::standard_bus(12_000.0, 5.9),
        opts_bevy::PlayerVehicle,
    ));
    app.update();
    app.update();
    assert_eq!(display_of(&mut app, "speed"), Some(Display::Flex));
}

/// No map, vehicle or place name may be written into the HUD or the loading screen: they come from
/// the session (M-0075, M-0076).
#[test]
fn no_map_or_vehicle_specific_literal_in_the_hud_and_loading_sources() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec![
        root.join("in_game/hud.rs"),
        root.join("in_game/loading_ui.rs"),
        root.join("in_game/loading_pipeline.rs"),
    ];
    for entry in std::fs::read_dir(root.join("hud"))
        .expect("hud dir")
        .flatten()
    {
        let p = entry.path();
        if p.extension().is_some_and(|e| e == "rs") && !p.ends_with("tests.rs") {
            files.push(p);
        }
    }
    let banned = [
        "Westerwald",
        "Siegener",
        "Spandau",
        "Berlin",
        "SD77",
        "SD200",
        "MAN ",
    ];
    for f in files {
        let text = std::fs::read_to_string(&f).expect("source");
        let code = text.split("#[cfg(test)]").next().unwrap_or("");
        for (n, line) in code.lines().enumerate() {
            for b in banned {
                assert!(
                    !line.contains(b),
                    "{}:{}: map- or vehicle-specific literal `{b}`: {line}",
                    f.display(),
                    n + 1
                );
            }
        }
    }
}
