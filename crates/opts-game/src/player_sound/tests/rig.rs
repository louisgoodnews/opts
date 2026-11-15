//! Test rig: a Bevy app with one vehicle, its sound configuration and a temporary sound file.

use super::*;

pub(super) fn wav() -> Vec<u8> {
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

pub(super) struct Rig {
    pub(super) app: App,
    pub(super) root: Entity,
    pub(super) dir: std::path::PathBuf,
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub(super) fn rig(name: &str, cfg: &str, script: &str) -> Rig {
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
pub(super) fn run(app: &mut App, mut done: impl FnMut(&App) -> bool) -> bool {
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

pub(super) fn emitters(app: &mut App) -> Vec<SoundEmitter> {
    app.world_mut()
        .query::<&SoundEmitter>()
        .iter(app.world())
        .cloned()
        .collect()
}

pub(super) fn emitters_of(app: &App) -> Vec<SoundEmitter> {
    let mut q = app.world().try_query::<&SoundEmitter>().expect("query");
    q.iter(app.world()).cloned().collect()
}

pub(super) fn set(rig: &mut Rig, name: &str, v: f32) {
    let mut q = rig.app.world_mut().query::<&VehicleScripts>();
    let s = q.single(rig.app.world()).expect("scripts");
    s.engine.lock().expect("lock").set(name, v);
}
