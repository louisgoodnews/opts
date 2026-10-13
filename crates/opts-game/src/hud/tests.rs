//! HUD tests: registry and profile consistency, the switches, focus, alerts, the camera label,
//! the developer panel and the Details dialog.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use opts_bevy::{CameraMode, CameraState};
use opts_gui::components::toast::{M3Toast, ToastActionButton, ToastSeverity};
use opts_gui::interaction::activation::Activated;
use opts_gui::interaction::focus::{Focusable, PointerOnly};
use opts_gui::prelude::*;

use super::build::{HudRoot, HudSlot, HudText, HudWidgetNode};
use super::fade::{FadeKind, HudFade, CAMERA_LABEL_SECONDS};
use super::profile::{unplaced, HudProfile, HudProfiles, ProfileProblem};
use super::update::CameraLabel;
use super::widget::{Binding, Slot, VehicleKind, WIDGETS};
use super::*;
use crate::app_state::AppState;
use crate::in_game::omsi_world::WorldStats;
use crate::in_game::types::InGameSessionInfo;
use crate::settings::GameSettings;
use crate::shell::ShellPlugin;

fn app() -> App {
    crate::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::dark())
        .init_state::<AppState>()
        .init_state::<opts_bevy::reporting::ReportModalState>()
        .init_resource::<opts_bevy::reporting::ReportModalData>()
        .init_resource::<CameraState>()
        .init_resource::<InGameSessionInfo>()
        .init_resource::<WorldStats>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(HudPlugin);
    // A drive with a vehicle: without one the HUD shows no instruments.
    app.world_mut().spawn((
        opts_bevy::VehicleController::standard_bus(12_000.0, 5.9),
        opts_bevy::PlayerVehicle,
    ));
    app.update();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    app
}

fn build(app: &mut App, kind: VehicleKind) {
    let profile = app
        .world()
        .resource::<HudProfiles>()
        .get(kind)
        .cloned()
        .expect("profile");
    let palette = MaterialTheme::dark()
        .instrument(opts_gui::theme::InstrumentVariant::Day)
        .to_owned();
    let mut commands = app.world_mut().commands();
    build::spawn_hud(&mut commands, &profile, &palette);
    app.update();
    app.update();
}

fn display_of(app: &mut App, id: &str) -> Option<Display> {
    app.world_mut()
        .query::<(&HudWidgetNode, &Node)>()
        .iter(app.world())
        .find(|(w, _)| w.id == id)
        .map(|(_, n)| n.display)
}

#[test]
fn every_widget_is_in_a_shipped_profile_and_the_profiles_are_clean() {
    let profiles = HudProfiles::load(None);
    assert_eq!(profiles.0.len(), VehicleKind::ALL.len());
    assert!(unplaced(&profiles).is_empty(), "{:?}", unplaced(&profiles));
    for kind in VehicleKind::ALL {
        let (p, problems) = HudProfile::parse(profile::shipped(kind)).expect("parses");
        assert_eq!(p.kind, kind);
        assert!(problems.is_empty(), "{}: {problems:?}", kind.id());
    }
    let mut ids: Vec<_> = WIDGETS.iter().map(|w| w.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), WIDGETS.len(), "widget ids are unique");
}

#[test]
fn the_tram_profile_has_no_rpm_and_orders_the_brake_before_the_gear() {
    let profiles = HudProfiles::load(None);
    let tram = profiles.get(VehicleKind::Tram).expect("tram");
    let ids: Vec<_> = tram
        .in_slot(Slot::Instruments)
        .iter()
        .map(|(_, w)| w.id)
        .collect();
    assert_eq!(ids, ["speed", "parking-brake", "gear"]);
}

#[test]
fn bad_profile_entries_are_dropped_and_reported() {
    let text = r#"HudProfile(kind: Bus, widgets: [
        (id: "speed"), (id: "speed"), (id: "nope"), (id: "gear", slot: Some(Hints)),
    ])"#;
    let (p, problems) = HudProfile::parse(text).expect("parses");
    assert_eq!(p.widgets.len(), 1);
    assert!(problems.contains(&ProfileProblem::Duplicate("speed".into())));
    assert!(problems.contains(&ProfileProblem::UnknownWidget("nope".into())));
    assert!(problems.contains(&ProfileProblem::SlotNotAllowed("gear".into(), Slot::Hints)));
    assert!(matches!(
        HudProfile::parse("not ron"),
        Err(ProfileProblem::Syntax(_))
    ));
}

#[test]
fn a_user_profile_replaces_the_shipped_one_and_a_broken_one_is_ignored() {
    let dir = std::env::temp_dir().join(format!("opts-hud-profiles-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(
        dir.join("bus.ron"),
        r#"HudProfile(kind: Bus, widgets: [(id: "speed")])"#,
    )
    .expect("write");
    std::fs::write(dir.join("tram.ron"), "garbage").expect("write");
    std::fs::write(
        dir.join("trolleybus.ron"),
        r#"HudProfile(kind: Bus, widgets: [])"#,
    )
    .expect("write");
    let profiles = HudProfiles::load(Some(&dir));
    assert_eq!(
        profiles.get(VehicleKind::Bus).map(|p| p.widgets.len()),
        Some(1)
    );
    assert!(profiles
        .get(VehicleKind::Tram)
        .is_some_and(|p| p.widgets.len() > 3));
    assert!(profiles
        .get(VehicleKind::Trolleybus)
        .is_some_and(|p| p.widgets.len() > 3));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_widget_switch_is_a_registered_setting_with_matching_default() {
    let registry = crate::settings_registry::SettingsRegistry::with_builtin();
    for w in WIDGETS {
        let Some(sw) = w.switch else { continue };
        let d = registry
            .get(sw.key)
            .unwrap_or_else(|| panic!("{} missing", sw.key));
        assert_eq!(d.default == "true", sw.default_on, "{}", sw.key);
        assert_eq!(d.label_key, sw.label_key);
    }
}

#[test]
fn every_hud_message_exists_in_english_and_german() {
    let mut missing = Vec::new();
    for locale in ["en", "de"] {
        let mut loc = crate::settings_registry::localization(locale);
        add_messages(&mut loc);
        let mut all: Vec<&str> = keys::ALL.to_vec();
        for w in WIDGETS {
            all.push(w.title_key);
        }
        for k in all {
            if loc.t(k).starts_with('⟦') {
                missing.push(format!("{locale}:{k}"));
            }
        }
    }
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn the_hud_has_no_focusable_element_except_pointer_only_chrome() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<HudRoot>>()
        .iter(app.world())
        .next()
        .expect("root");
    let mut checked = 0;
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        let world = app.world();
        if world.get::<Focusable>(e).is_some() {
            assert!(world.get::<PointerOnly>(e).is_some(), "{e:?} takes focus");
        }
        checked += 1;
        if let Some(c) = world.get::<Children>(e) {
            stack.extend(c.iter());
        }
    }
    assert!(checked > 15, "walked the HUD ({checked} entities)");
}

#[test]
fn the_cluster_is_built_from_the_profile_and_rpm_is_hidden_without_a_value() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    for id in ["speed", "gear", "parking-brake"] {
        assert_eq!(display_of(&mut app, id), Some(Display::Flex), "{id}");
    }
    assert_eq!(display_of(&mut app, "rpm"), Some(Display::None));
}

#[test]
fn switching_a_widget_off_removes_it_and_an_empty_slot_leaves_the_layout() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("hud.gear", "false");
    app.update();
    assert_eq!(display_of(&mut app, "gear"), Some(Display::None));
    assert_eq!(display_of(&mut app, "speed"), Some(Display::Flex));
    for key in ["hud.speed", "hud.parking_brake"] {
        app.world_mut()
            .resource_mut::<GameSettings>()
            .set_raw(key, "false");
    }
    app.update();
    let cluster = app
        .world_mut()
        .query::<(&HudSlot, &Node)>()
        .iter(app.world())
        .find(|(s, _)| s.0 == Slot::Instruments)
        .map(|(_, n)| n.display);
    assert_eq!(cluster, Some(Display::None));
}

#[test]
fn the_alert_slot_has_a_fixed_height_and_the_alert_cannot_be_switched_off() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let fixed = app
        .world_mut()
        .query::<(&Node, &Children)>()
        .iter(app.world())
        .any(|(n, c)| {
            n.height == Val::Px(44.0) && c.iter().any(|e| app.world().get::<HudSlot>(e).is_some())
        });
    assert!(fixed, "the alert line sits in a box of fixed height");
    assert!(widget::widget("alerts").is_some_and(|w| w.switch.is_none()));
}

#[test]
fn an_alert_shows_in_the_slot_and_critical_stays() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut().write_message(RaiseAlert {
        id: "t",
        severity: AlertSeverity::Critical,
        key: keys::ALERT_TEST,
        args: Vec::new(),
    });
    app.update();
    app.update();
    let line = |app: &mut App| {
        app.world_mut()
            .query::<(&build::HudText, &Text)>()
            .iter(app.world())
            .find(|(h, _)| h.binding == Binding::Alerts)
            .map(|(_, t)| t.0.clone())
    };
    let text = line(&mut app).unwrap_or_default();
    assert!(text.starts_with("Alert:"), "{text}");
    let visible = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Visibility, With<build::AlertPill>>()
            .iter(app.world())
            .next()
            .copied()
    };
    assert_eq!(visible(&mut app), Some(Visibility::Inherited));
    app.world_mut().write_message(ClearAlert("t"));
    app.update();
    assert_eq!(visible(&mut app), Some(Visibility::Hidden));
}

#[test]
fn a_camera_change_shows_the_label_for_one_and_a_half_seconds() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let alpha = |app: &mut App| {
        app.world_mut()
            .query::<(&FadeKind, &HudFade)>()
            .iter(app.world())
            .find(|(k, _)| **k == FadeKind::CameraLabel)
            .map(|(_, f)| f.alpha(false))
    };
    assert_eq!(alpha(&mut app), Some(0.0), "no label at the start");
    assert!(app.world().resource::<CameraLabel>().shown.is_none());
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    app.update();
    app.update();
    assert_eq!(alpha(&mut app), Some(1.0));
    // advance the fade by hand: 1.6 s later it is gone
    for (k, mut f) in app
        .world_mut()
        .query::<(&FadeKind, &mut HudFade)>()
        .iter_mut(app.world_mut())
    {
        if *k == FadeKind::CameraLabel {
            f.elapsed = CAMERA_LABEL_SECONDS + 0.1;
        }
    }
    assert_eq!(alpha(&mut app), Some(0.0));
    let text = app
        .world_mut()
        .query::<(&build::HudText, &Text)>()
        .iter(app.world())
        .find(|(h, _)| h.binding == Binding::CameraLabel)
        .map(|(_, t)| t.0.clone());
    assert_eq!(text.as_deref(), Some("Driver's view"));
}

#[test]
fn the_status_bar_is_pinned_while_the_game_is_paused() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.update();
    let pinned = app
        .world_mut()
        .query::<(&FadeKind, &HudFade)>()
        .iter(app.world())
        .find(|(k, _)| **k == FadeKind::Status)
        .map(|(_, f)| f.pinned);
    assert_eq!(pinned, Some(true));
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    app.update();
    let f = app
        .world_mut()
        .query::<(&FadeKind, &HudFade)>()
        .iter(app.world())
        .find(|(k, _)| **k == FadeKind::Status)
        .map(|(_, f)| (f.pinned, f.elapsed));
    assert!(matches!(f, Some((false, e)) if e < 1.0));
}

#[test]
fn speed_follows_the_unit_setting() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut().resource_mut::<HudValues>().speed_kmh = 50.0;
    // `sample_values` would reset the values without a vehicle; read the texts right after
    // the update systems by running the text system alone.
    let speed = |app: &mut App| {
        app.world_mut()
            .run_system_once(update::update_texts)
            .expect("runs");
        app.world_mut()
            .query::<(&build::HudText, &Text)>()
            .iter(app.world())
            .filter(|(h, _)| h.binding == Binding::Speed)
            .map(|(_, t)| t.0.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(speed(&mut app), ["50", "KM/H"]);
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("interface.units", "mph");
    assert_eq!(speed(&mut app), ["31", "MPH"]);
}

#[test]
fn the_developer_panel_is_off_by_default_and_f10_changes_its_setting() {
    use crate::settings_registry::{SettingChanged, SettingsCapabilities, SettingsRegistry};
    let dir = std::env::temp_dir().join(format!("opts-hud-dev-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let mut settings = GameSettings::default();
    settings.db_path_override = Some(dir.join("settings.db"));
    let mut app = App::new();
    app.insert_resource(settings)
        .insert_resource(SettingsRegistry::with_builtin())
        .init_resource::<SettingsCapabilities>()
        .add_message::<SettingChanged>()
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(WorldStats::default())
        .add_systems(
            Update,
            (
                crate::in_game::omsi_world::toggle_developer_panel,
                crate::settings_registry::live::sync_developer_panel,
            )
                .chain(),
        );
    app.update();
    assert!(!app.world().resource::<WorldStats>().overlay_visible);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F10);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    assert_eq!(
        app.world()
            .resource::<GameSettings>()
            .raw("advanced.developer_panel")
            .as_deref(),
        Some("true")
    );
    app.update();
    assert!(app.world().resource::<WorldStats>().overlay_visible);
    let _ = std::fs::remove_dir_all(&dir);
}

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
