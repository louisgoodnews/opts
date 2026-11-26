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
use opts_sim::script::{FileRead, ScriptEngine};

use self::consts::{KEYBOARD, MODEL, SCRIPT};

/// Reads a script file that the install lookup found: the loader only needs the outcome.
fn read_script(path: Option<std::path::PathBuf>) -> FileRead {
    let Some(path) = path else {
        return FileRead::NotFound;
    };
    match std::fs::read(&path) {
        Ok(bytes) => FileRead::Loaded {
            name: path.to_string_lossy().into_owned(),
            bytes,
        },
        Err(e) => FileRead::Unreadable(e.to_string()),
    }
}

#[path = "vehicle_script_bridge/consts.rs"]
mod consts;
#[path = "vehicle_script_bridge/corpus.rs"]
mod corpus;
#[path = "vehicle_script_bridge/headless.rs"]
mod headless;
#[path = "vehicle_script_bridge/nlc.rs"]
mod nlc;
#[path = "vehicle_script_bridge/real.rs"]
mod real;

#[path = "vehicle_script_bridge/corpus_support.rs"]
mod corpus_support;

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
    let anim = MeshAnimation::from_mesh(root, mesh_def, None).expect("mesh is animated");
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
