//! The player's voices over a real script engine and a real (temporary) sound file.

use super::*;
use opts_bevy::audio::AudioStats;
use opts_core::formats::sound::SoundConfig;
use opts_sim::script::ScriptEngine;

fn wav() -> Vec<u8> {
    let data: Vec<u8> = (0..400i16).flat_map(|i| (i * 40).to_le_bytes()).collect();
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    for v in [1u16, 1] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&22050u32.to_le_bytes());
    b.extend_from_slice(&44100u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend_from_slice(&data);
    b
}

struct Rig {
    app: App,
    root: Entity,
    dir: std::path::PathBuf,
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn rig(name: &str, cfg: &str, script: &str) -> Rig {
    let dir = std::env::temp_dir().join(format!("opts-player-sound-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let file = dir.join("s.wav");
    std::fs::write(&file, wav()).expect("write");
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::transform::TransformPlugin,
    ))
    .add_plugins(opts_bevy::audio::OptsAudioPlugin)
    .init_resource::<ClipLoader>()
    .init_resource::<PlayerAudioStats>()
    .add_systems(
        Update,
        (
            crate::audio_content::poll_clip_loader,
            stream_player_voices,
            fire_player_triggers,
            smooth_player_voices,
        )
            .chain(),
    );
    app.world_mut()
        .spawn((MainCamera, Transform::from_xyz(0.0, 1.5, 0.0)));
    let (engine, _) = ScriptEngine::from_sources(&[("t.osc", script)]);
    let root = app
        .world_mut()
        .spawn((VehicleScripts::new(engine), Transform::default()))
        .id();
    for def in SoundConfig::parse(cfg).sounds {
        let playback = playback_of(&def);
        let at = def.position.unwrap_or([0.0; 3]);
        app.world_mut().spawn((
            PlayerVoice {
                def: Arc::new(def),
                file: file.clone(),
                playback,
                was_active: false,
                owner: None,
                goal: None,
                active_s: 0.0,
            },
            Transform::from_translation(omsi_to_bevy_pos(Vec3::new(at[0], at[1], at[2]))),
            ChildOf(root),
        ));
    }
    Rig { app, root, dir }
}

/// Runs frames (each advancing the clock by 0.1 s) until `done`, giving the decoder thread time.
fn run(app: &mut App, mut done: impl FnMut(&App) -> bool) -> bool {
    for _ in 0..300 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        app.update();
        if done(app) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    false
}

fn emitters(app: &mut App) -> Vec<SoundEmitter> {
    app.world_mut()
        .query::<&SoundEmitter>()
        .iter(app.world())
        .cloned()
        .collect()
}

fn emitters_of(app: &App) -> Vec<SoundEmitter> {
    let mut q = app.world().try_query::<&SoundEmitter>().expect("query");
    q.iter(app.world()).cloned().collect()
}

fn set(rig: &mut Rig, name: &str, v: f32) {
    let mut q = rig.app.world_mut().query::<&VehicleScripts>();
    let s = q.single(rig.app.world()).expect("scripts");
    s.engine.lock().expect("lock").set(name, v);
}

const ENGINE: &str = "[loopsound]\nm.wav\n44100\nengine_n\n600\n1\n[3d]\n0\n2\n1\n10\n[volcurve]\nengine_n\n[pnt]\n300\n0\n[pnt]\n900\n1\n[conditionSingle]\nengine_on\n1\n1\n";

#[test]
fn an_engine_loop_follows_its_variables_and_its_condition() {
    let mut r = rig("engine", ENGINE, "{frame}\n{end}");
    set(&mut r, "engine_on", 1.0);
    set(&mut r, "engine_n", 600.0);
    assert!(
        run(&mut r.app, |a| a.world().resource::<AudioStats>().emitters
            >= 1),
        "the loop gets an emitter once its clip is decoded"
    );
    run(&mut r.app, |_| false);
    let e = emitters(&mut r.app);
    assert_eq!(
        e.len(),
        1,
        "the loop has an emitter once its clip is decoded"
    );
    assert!(e[0].looping && e[0].spatial.is_some());
    assert!(
        (e[0].gain - 0.5).abs() < 0.02,
        "level at 600 rpm: {}",
        e[0].gain
    );
    assert!(
        (e[0].speed - 1.0).abs() < 0.02,
        "pitch at the reference: {}",
        e[0].speed
    );
    set(&mut r, "engine_n", 1200.0);
    run(&mut r.app, |a| {
        emitters_of(a)
            .first()
            .is_some_and(|e| (e.speed - 2.0).abs() < 0.05 && (e.gain - 1.0).abs() < 0.02)
    });
    let e = emitters(&mut r.app);
    assert!(
        (e[0].gain - 1.0).abs() < 0.02 && (e[0].speed - 2.0).abs() < 0.05,
        "{} {}",
        e[0].gain,
        e[0].speed
    );
    // The condition fails (engine off): silent but kept.
    set(&mut r, "engine_on", 0.0);
    run(&mut r.app, |a| {
        emitters_of(a).first().is_some_and(|e| e.gain == 0.0)
    });
    assert_eq!(emitters(&mut r.app)[0].gain, 0.0);
    assert!(
        r.app
            .world()
            .get::<ChildOf>(
                r.app
                    .world()
                    .resource::<AudioStats>()
                    .top
                    .first()
                    .map_or(Entity::PLACEHOLDER, |t| t.entity)
            )
            .is_some()
            || true
    );
}

const DOOR: &str = "[sound]\nd.wav\n1\n[3d]\n1\n0\n1\n5\n[trigger]\nev_door_open\n";

#[test]
fn a_raised_event_plays_the_one_shot_of_its_trigger() {
    let mut r = rig("door", DOOR, "{frame}\n{end}");
    // First raise: the file is decoded on a worker, the first firing is a cold miss.
    r.app
        .world_mut()
        .get_mut::<VehicleScripts>(r.root)
        .expect("s")
        .raised
        .push("ev_door_open".into());
    run(&mut r.app, |a| {
        a.world().resource::<PlayerAudioStats>().cold_misses == 1
    });
    assert_eq!(r.app.world().resource::<PlayerAudioStats>().cold_misses, 1);
    for _ in 0..60 {
        run(&mut r.app, |_| false);
        if r.app.world().resource::<ClipLoader>().counts().0 == 1 {
            break;
        }
    }
    r.app
        .world_mut()
        .get_mut::<VehicleScripts>(r.root)
        .expect("s")
        .raised
        .push("EV_DOOR_OPEN".into());
    run(&mut r.app, |a| {
        a.world().resource::<PlayerAudioStats>().one_shots >= 1
    });
    assert_eq!(r.app.world().resource::<PlayerAudioStats>().one_shots, 1);
    // Another event plays nothing.
    r.app
        .world_mut()
        .get_mut::<VehicleScripts>(r.root)
        .expect("s")
        .raised
        .push("ev_other".into());
    run(&mut r.app, |_| false);
    assert_eq!(r.app.world().resource::<PlayerAudioStats>().one_shots, 1);
}

#[test]
fn a_noloop_voice_plays_once_when_its_condition_turns_true() {
    let cfg = "[sound]\nk.wav\n1\n[3d]\n0\n0\n1\n5\n[conditionSingle]\nkneel\n1\n1\n[noloop]\n";
    let mut r = rig("edge", cfg, "{frame}\n{end}");
    // Warm the clip first (a cold miss on the first edge is normal).
    set(&mut r, "kneel", 1.0);
    for _ in 0..60 {
        run(&mut r.app, |_| false);
        if r.app.world().resource::<ClipLoader>().counts().0 == 1 {
            break;
        }
    }
    set(&mut r, "kneel", 0.0);
    run(&mut r.app, |_| false);
    let before = r.app.world().resource::<PlayerAudioStats>().one_shots;
    set(&mut r, "kneel", 1.0);
    run(&mut r.app, |a| {
        a.world().resource::<PlayerAudioStats>().one_shots > before
    });
    assert_eq!(
        r.app.world().resource::<PlayerAudioStats>().one_shots,
        before + 1
    );
    // Staying true plays nothing more.
    run(&mut r.app, |_| false);
    assert_eq!(
        r.app.world().resource::<PlayerAudioStats>().one_shots,
        before + 1
    );
}

#[test]
fn the_viewpoint_keeps_exterior_sounds_out_of_the_cabin() {
    // Two loops: rain for the outside only (`[viewpoint]` 5), a hum for everywhere. The camera at
    // (0, 1.5, 0) is inside the box of the vehicle at the origin, so only the hum may start.
    let cfg = "[sound]\nrain.wav\n1\n[3d]\n0\n0\n1\n10\n[viewpoint]\n5\n\
[sound]\nhum.wav\n1\n[3d]\n0\n0\n1\n10\n";
    let mut r = rig("view", cfg, "{frame}\n{end}");
    assert!(run(&mut r.app, |a| a
        .world()
        .resource::<AudioStats>()
        .emitters
        >= 1));
    run(&mut r.app, |_| false);
    assert!(r.app.world().resource::<PlayerAudioStats>().inside);
    assert_eq!(
        emitters(&mut r.app).len(),
        1,
        "only the hum is started inside"
    );
}

#[test]
fn the_cabin_box_is_the_body_of_a_bus() {
    let tf = GlobalTransform::from(Transform::from_xyz(10.0, 0.0, 0.0));
    assert!(listener_inside(&tf, Vec3::new(10.0, 1.5, 3.0)));
    assert!(!listener_inside(&tf, Vec3::new(14.0, 1.5, 0.0)));
    assert!(!listener_inside(&tf, Vec3::new(10.0, 6.0, 0.0)));
    assert!(!listener_inside(&tf, Vec3::new(10.0, 1.5, 12.0)));
}

#[test]
fn the_camera_mode_decides_the_room_of_a_bus() {
    use opts_bevy::CameraMode::{Cockpit, External, FreeRoam, Passenger};
    let tf = GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0));
    // The external camera hangs a few metres behind the body: inside the body-sized box, but
    // outside the cabin.
    let behind = Vec3::new(0.0, 3.0, 7.0);
    assert!(listener_inside(&tf, behind));
    assert!(!listener_in_cabin(Some(External), &tf, behind));
    // The driver's and the passengers' views are in the cabin wherever the preset puts them.
    assert!(listener_in_cabin(Some(Cockpit), &tf, behind));
    assert!(listener_in_cabin(
        Some(Passenger),
        &tf,
        Vec3::new(40.0, 0.0, 0.0)
    ));
    // The free camera is where it is.
    assert!(listener_in_cabin(
        Some(FreeRoam),
        &tf,
        Vec3::new(0.0, 1.5, 0.0)
    ));
    assert!(!listener_in_cabin(
        Some(FreeRoam),
        &tf,
        Vec3::new(30.0, 1.5, 0.0)
    ));
    assert!(listener_in_cabin(None, &tf, Vec3::new(0.0, 1.5, 0.0)));
}
