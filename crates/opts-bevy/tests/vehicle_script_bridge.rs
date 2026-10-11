//! Headless test of the active script bridge (M-0045 Phase C): a key raises a script trigger,
//! the script writes variables, and those move a `[newanim]` mesh and switch the vehicle's
//! interior light and spotlights.

use bevy::prelude::*;
use opts_bevy::{
    script_key_trigger_system, spawn_vehicle_model, vehicle_animation_system, vehicle_light_system,
    LightSwitch, MeshAnimation, PlayerVehicle, ScriptKeyBindings, ScriptLight, VehicleScripts,
};
use opts_core::formats::engine_config::KeyboardConfig;
use opts_core::formats::vehicles::bus::BusManifest;
use opts_core::formats::vehicles::model::ModelConfig;
use opts_sim::script::ScriptEngine;

const MODEL: &str = "\
[mesh]
door.o3d

[mouseevent]
bus_doorfront0

[newanim]
origin_trans
0
0
0
anim_trans
door_0
0.8

[interiorlight]
lights_cab
0.7
255
255
255
0
3
2

[spotlight]
0
2
0.8
0
1
-0.3
233
233
255
500
30
70

[spotlight]
0
2
0.8
0
1
-0.3
255
255
233
100
30
70
";

const SCRIPT: &str = "\
{init}
    -1 (S.L.Spot_Select)
{end}

{trigger:bus_doorfront0}
    1 (S.L.door_0)
    1 (S.L.lights_cab)
    1 (S.L.Spot_Select)
{end}

{trigger:horn_off}
    0 (S.L.door_0)
{end}
";

const KEYBOARD: &[u8] = b"[vehicles]\n[entry]\nbus_doorfront0\n181\n0\n[entry]\nhorn\n35\n1\n";

fn app() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(ScriptKeyBindings::from_config(&KeyboardConfig::parse(
        KEYBOARD,
    )));
    app.add_systems(
        Update,
        (
            script_key_trigger_system,
            vehicle_animation_system,
            vehicle_light_system,
        )
            .chain(),
    );

    let model = ModelConfig::parse(MODEL).expect("model parses");
    let manifest = BusManifest::parse("[friendlyname]\nTest\nBus\nTest\n").expect("bus parses");
    let (mut engine, diags) = ScriptEngine::from_sources(&[("test.osc", SCRIPT)]);
    assert!(
        diags
            .iter()
            .all(|d| d.severity != opts_sim::script::Severity::Error),
        "{diags:?}"
    );
    engine.run_init();

    let world = app.world_mut();
    let mut commands = world.commands();
    let root = spawn_vehicle_model(
        &mut commands,
        &manifest,
        &model,
        Transform::IDENTITY,
        &[],
        &[],
    );
    world.flush();
    world
        .entity_mut(root)
        .insert((VehicleScripts::new(engine), PlayerVehicle));
    let mesh_def = &model.lods[0].meshes[0];
    assert_eq!(mesh_def.mouseevent.as_deref(), Some("bus_doorfront0"));
    let anim = MeshAnimation::from_mesh(root, mesh_def).expect("mesh is animated");
    let door = world
        .spawn((anim, Transform::IDENTITY, Visibility::Inherited))
        .id();
    (app, root, door)
}

fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
}

/// `(intensity, visible)` of every light of the vehicle, interior light first, then the
/// spotlights in declaration order.
fn lights(app: &mut App) -> Vec<(f32, bool)> {
    let world = app.world_mut();
    let mut q = world.query::<(
        &ScriptLight,
        Option<&PointLight>,
        Option<&SpotLight>,
        &Visibility,
    )>();
    let mut out: Vec<(usize, f32, bool)> = q
        .iter(world)
        .map(|(l, p, s, v)| {
            let order = match l.switch {
                LightSwitch::Variable(_) => 0,
                LightSwitch::Spot(i) => i + 1,
            };
            let intensity = p.map(|p| p.intensity).or(s.map(|s| s.intensity)).unwrap();
            (order, intensity, *v != Visibility::Hidden)
        })
        .collect();
    out.sort_by_key(|l| l.0);
    out.into_iter().map(|(_, i, v)| (i, v)).collect()
}

#[test]
fn lights_start_dark_and_the_door_closed() {
    let (mut app, _, door) = app();
    app.update();
    let lights = lights(&mut app);
    assert_eq!(lights.len(), 3);
    assert!(lights.iter().all(|(i, v)| *i == 0.0 && !v), "{lights:?}");
    let tf = app.world().get::<Transform>(door).unwrap();
    assert!(tf.translation.length() < 1e-6);
}

#[test]
fn a_key_raises_the_trigger_that_opens_the_door_and_switches_lights() {
    let (mut app, _, door) = app();
    app.update();
    press(&mut app, KeyCode::NumpadDivide);

    // `anim_trans door_0 0.8` along the frame's X axis (OMSI x = Bevy x).
    let tf = *app.world().get::<Transform>(door).unwrap();
    assert!(
        (tf.translation - Vec3::new(0.8, 0.0, 0.0)).length() < 1e-5,
        "{tf:?}"
    );

    let lights = lights(&mut app);
    // Interior light on at factor 1; of the spotlights only number 1 (Spot_Select = 1).
    assert!(lights[0].0 > 0.0 && lights[0].1, "{lights:?}");
    assert!(lights[1].0 == 0.0 && !lights[1].1, "{lights:?}");
    assert!(lights[2].0 > 0.0 && lights[2].1, "{lights:?}");
}

#[test]
fn releasing_a_held_key_raises_its_off_trigger() {
    let (mut app, _, door) = app();
    app.update();
    press(&mut app, KeyCode::NumpadDivide);
    // `horn` is a held binding: its release raises `horn_off`, which closes the door here.
    press(&mut app, KeyCode::KeyH);
    app.update();
    let tf = app.world().get::<Transform>(door).unwrap();
    assert!(tf.translation.length() < 1e-5, "{tf:?}");
}

/// Real installation (skipped unless `OMSI_PATH` is set): every `[vehicles]` entry of
/// `keyboard.cfg` has a known key, and the SD77's scripts handle the triggers that its keys
/// and `[mouseevent]` hot-spots raise.
#[test]
fn real_keyboard_cfg_and_sd77_triggers() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let install = opts_bevy::OmsiInstall::new(root.clone());
    let cfg_path = install
        .find_file("Inputs/keyboard.cfg")
        .expect("Inputs/keyboard.cfg");
    let cfg = KeyboardConfig::parse(&std::fs::read(cfg_path).unwrap());
    let vehicle: Vec<_> = cfg.vehicle_bindings().collect();
    assert!(vehicle.len() > 50, "only {} vehicle entries", vehicle.len());
    for b in &vehicle {
        assert!(
            opts_bevy::dik_to_keycode(b.scan_code).is_some(),
            "{}: unknown scan code {}",
            b.action,
            b.scan_code
        );
    }
    let bindings = ScriptKeyBindings::from_config(&cfg);
    eprintln!(
        "{} vehicle entries, {} bound",
        vehicle.len(),
        bindings.0.len()
    );

    let Some(bus_path) = install.find_file("Vehicles/MAN_SD200/MAN_SD77.bus") else {
        eprintln!("MAN_SD200 not installed; skipping the trigger check");
        return;
    };
    let bus_dir = bus_path.parent().unwrap().to_path_buf();
    let manifest = BusManifest::parse_bytes(&std::fs::read(&bus_path).unwrap()).unwrap();
    let (mut engine, _) =
        opts_sim::script::load_vehicle_scripts(&manifest, |rel| install.find_in(&bus_dir, rel));
    engine.run_init();
    for name in [
        "bus_doorfront0",
        "bus_dooraft",
        "kw_scheinwerfer_toggle",
        "horn",
    ] {
        assert!(engine.trigger(name), "SD77 has no trigger {name}");
    }
}
