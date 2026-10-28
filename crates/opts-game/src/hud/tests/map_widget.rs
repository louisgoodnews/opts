//! The mini-map widget: rows, redraws, centring, keys and radius.

use super::*;

/// A drive with the HUD, a vehicle with a position, fixture roads, a registry for the settings
/// (saved under a throwaway folder) and a clock that advances 16 ms per frame.
pub(super) fn map_app() -> App {
    use crate::settings_registry::{SettingChanged, SettingsCapabilities, SettingsRegistry};
    let dir = std::env::temp_dir().join(format!("opts-hud-map-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let mut app = app();
    {
        let mut settings = app.world_mut().resource_mut::<GameSettings>();
        settings.db_path_override = Some(dir.join("settings.db"));
    }
    app.insert_resource(SettingsRegistry::with_builtin())
        .init_resource::<SettingsCapabilities>()
        .add_message::<SettingChanged>()
        .insert_resource(FixtureRoads::grid())
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(16),
        ));
    // The vehicle stands at (30 east, 20 north), heading east.
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::PlayerVehicle>>();
    let player = q.iter(app.world()).next().expect("player");
    app.world_mut().entity_mut(player).insert((
        GlobalTransform::from(
            Transform::from_xyz(30.0, 0.0, -20.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        ),
        crate::session_vehicles::SessionVehicle(1),
    ));
    app.world_mut().spawn((
        crate::session_vehicles::SessionVehicle(2),
        GlobalTransform::from(Transform::from_xyz(-60.0, 0.0, 10.0)),
    ));
    build(&mut app, VehicleKind::Bus);
    let profile = app
        .world()
        .resource::<HudProfiles>()
        .get(VehicleKind::Bus)
        .cloned()
        .expect("profile");
    app.insert_resource(build::ActiveProfile {
        kind: VehicleKind::Bus,
        profile,
    });
    app
}

fn minimap_state(app: &App) -> MiniMapState {
    app.world().resource::<MiniMapState>().clone()
}

pub(super) fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

#[test]
fn the_cluster_has_its_rows_in_a_fixed_order_and_the_map_is_a_separate_slot() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let cluster = app
        .world_mut()
        .query::<(Entity, &build::HudSlot)>()
        .iter(app.world())
        .find(|(_, s)| s.0 == Slot::ClusterBottomLeft)
        .map(|(e, _)| e)
        .expect("cluster");
    let rows: Vec<widget::Row> = app
        .world()
        .get::<Children>(cluster)
        .expect("children")
        .iter()
        .filter_map(|c| app.world().get::<build::HudRow>(c).map(|r| r.0))
        .collect();
    assert_eq!(
        rows,
        [
            widget::Row::Hints,
            widget::Row::Alerts,
            widget::Row::Instruments
        ]
    );
    // The map is a sibling of the cluster, after it (bottom right).
    let parent = app
        .world()
        .get::<ChildOf>(cluster)
        .expect("parent")
        .parent();
    let siblings: Vec<Entity> = app
        .world()
        .get::<Children>(parent)
        .expect("c")
        .iter()
        .collect();
    let map_slot = app
        .world_mut()
        .query::<(Entity, &build::HudSlot)>()
        .iter(app.world())
        .find(|(_, s)| s.0 == Slot::MiniMap)
        .map(|(e, _)| e)
        .expect("map slot");
    let at = |e: Entity| siblings.iter().position(|s| *s == e).expect("in the row");
    assert!(at(cluster) < at(map_slot));
}

#[test]
fn every_shipped_profile_places_the_map_and_the_rows_in_the_cluster() {
    let profiles = HudProfiles::load(None);
    for kind in VehicleKind::ALL {
        let p = profiles.get(kind).expect("profile");
        assert_eq!(p.in_slot(Slot::MiniMap).len(), 1, "{}", kind.id());
        for row in widget::Row::ALL {
            assert!(!p.in_row(row).is_empty(), "{} {}", kind.id(), row.id());
        }
        assert_eq!(
            p.in_slot(Slot::ClusterBottomLeft).len(),
            widget::Row::ALL
                .iter()
                .map(|r| p.in_row(*r).len())
                .sum::<usize>()
        );
    }
}

#[test]
fn the_map_follows_the_vehicle_marks_all_session_vehicles_and_draws_the_roads() {
    let mut app = map_app();
    run(&mut app, 60);
    let s = minimap_state(&app);
    assert!(s.frames > 0, "redrawn");
    let c = s.centre.expect("centre");
    assert!(
        (c.x - 30.0).abs() < 1e-3 && (c.y - 20.0).abs() < 1e-3,
        "{c:?}"
    );
    assert_eq!(s.markers, 2, "the active and the other session vehicle");
    assert!(s.lines >= 10, "roads: {}", s.lines);
    // Heading east: heading-up gives 90 degrees up.
    assert!(
        (s.up_heading_deg - 90.0).abs() < 0.01,
        "{}",
        s.up_heading_deg
    );
    // The texture really holds road pixels.
    let handle = app
        .world()
        .resource::<MiniMapImage>()
        .handle()
        .expect("image");
    let images = app.world().resource::<Assets<Image>>();
    let data = images
        .get(&handle)
        .expect("asset")
        .data
        .clone()
        .expect("data");
    let road =
        super::super::minimap::colors_of(&opts_gui::theme::InstrumentTokens::default().day).road;
    assert!(
        data.chunks_exact(4).any(|p| p == road.as_slice()),
        "roads in the texture"
    );
}

#[test]
fn the_map_redraws_at_most_ten_times_a_second() {
    let mut app = map_app();
    run(&mut app, 12);
    let before = minimap_state(&app).frames;
    // 100 frames of 16 ms are 1.6 s: at most 16 redraws (and about 14).
    run(&mut app, 100);
    let drawn = minimap_state(&app).frames - before;
    assert!((8..=16).contains(&drawn), "{drawn} redraws in 1.6 s");
}

#[test]
fn a_redraw_is_within_the_budget_on_the_fixture() {
    let mut app = map_app();
    run(&mut app, 60);
    let s = minimap_state(&app);
    // The slowest redraw includes the first one (cold caches); it stays well inside a frame.
    assert!(
        s.last_us < super::super::minimap::BUDGET_US * 4,
        "{} us",
        s.last_us
    );
}

#[test]
fn without_a_vehicle_the_map_is_centred_on_the_free_camera() {
    let mut app = map_app();
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::PlayerVehicle>>();
    let player = q.iter(app.world()).next().expect("player");
    app.world_mut().despawn(player);
    app.world_mut().spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        GlobalTransform::from(Transform::from_xyz(100.0, 5.0, -50.0)),
    ));
    // An auxiliary camera (a reflection mirror) is not the player's view.
    app.world_mut().spawn((
        Camera3d::default(),
        Name::new("MirrorCamera"),
        GlobalTransform::from(Transform::from_xyz(-900.0, 5.0, 700.0)),
    ));
    run(&mut app, 30);
    let s = minimap_state(&app);
    let c = s.centre.expect("centre");
    assert!(
        (c.x - 100.0).abs() < 1e-3 && (c.y - 50.0).abs() < 1e-3,
        "{c:?}"
    );
    // The camera is a marker, and so is the other session vehicle.
    assert_eq!(s.markers, 2);
}

#[test]
fn the_map_can_be_switched_off_and_the_widget_leaves_the_layout() {
    let mut app = map_app();
    run(&mut app, 30);
    assert_eq!(display_of(&mut app, "minimap"), Some(Display::Flex));
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw(SETTING_ENABLED, "false");
    run(&mut app, 3);
    assert_eq!(display_of(&mut app, "minimap"), Some(Display::None));
    let frames = minimap_state(&app).frames;
    run(&mut app, 60);
    assert_eq!(
        minimap_state(&app).frames,
        frames,
        "nothing is drawn while it is off"
    );
    // The slot is removed from the layout too.
    let slot = app
        .world_mut()
        .query::<(&build::HudSlot, &Node)>()
        .iter(app.world())
        .find(|(s, _)| s.0 == Slot::MiniMap)
        .map(|(_, n)| n.display);
    assert_eq!(slot, Some(Display::None));
}

#[test]
fn north_up_keeps_north_up() {
    let mut app = map_app();
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw(SETTING_ORIENTATION, "north");
    run(&mut app, 30);
    assert_eq!(minimap_state(&app).up_heading_deg, 0.0);
}

#[test]
fn the_keys_change_the_settings() {
    let mut app = map_app();
    let press = |app: &mut App, keys: &[KeyCode]| {
        use bevy::input::keyboard::{Key, KeyboardInput};
        use bevy::input::ButtonState;
        for state in [ButtonState::Pressed, ButtonState::Released] {
            for k in keys {
                app.world_mut().write_message(KeyboardInput {
                    key_code: *k,
                    logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                    state,
                    text: None,
                    repeat: false,
                    window: Entity::PLACEHOLDER,
                });
            }
            for _ in 0..2 {
                app.update();
            }
        }
    };
    let config = |app: &App| MiniMapConfig::read(app.world().resource::<GameSettings>());
    assert_eq!(config(&app).zoom_m, 200.0);
    press(&mut app, &[KeyCode::Equal]);
    assert_eq!(config(&app).zoom_m, 100.0, "+ zooms in");
    press(&mut app, &[KeyCode::Equal]);
    assert_eq!(config(&app).zoom_m, 100.0, "stops at 100 m");
    press(&mut app, &[KeyCode::Minus]);
    press(&mut app, &[KeyCode::Minus]);
    assert_eq!(config(&app).zoom_m, 500.0, "- zooms out");
    press(&mut app, &[KeyCode::ShiftLeft, KeyCode::F7]);
    assert_eq!(config(&app).orientation, Orientation::North);
    press(&mut app, &[KeyCode::F7]);
    assert!(!config(&app).enabled, "F7 hides it");
    press(&mut app, &[KeyCode::F7]);
    assert!(config(&app).enabled, "and shows it again");
    assert_eq!(
        config(&app).zoom_mode,
        minimap::ZoomMode::Speed,
        "speed zoom is the default"
    );
    press(&mut app, &[KeyCode::ControlLeft, KeyCode::F7]);
    assert_eq!(
        config(&app).zoom_mode,
        minimap::ZoomMode::Fixed,
        "Ctrl+F7 fixes the zoom"
    );
    assert!(config(&app).enabled, "and does not hide the map");
    press(&mut app, &[KeyCode::ControlLeft, KeyCode::F7]);
    assert_eq!(config(&app).zoom_mode, minimap::ZoomMode::Speed, "and back");
}

#[path = "map_widget/tests.rs"]
mod protocol_and_radius;
