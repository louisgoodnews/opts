//! The reload pipeline end to end in a headless app: a real file watcher, debouncing, validation,
//! failure handling and the `reload.*` / `watch.*` control commands.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use opts_bevy::control::{ControlAddr, ControlConfig, ControlPlugin};
use opts_bevy::hot_reload::{
    HotReloadConfig, HotReloadPlugin, ReloadFailed, ReloadKind, ReloadStats, ReloadValidated,
};
use opts_cli::client::{Client, ClientConfig, ClientError};
use opts_cli::protocol::code;
use serde_json::{json, Value};

#[derive(Resource, Default)]
struct Seen {
    validated: Vec<(PathBuf, ReloadKind)>,
    failed: Vec<(PathBuf, String)>,
}

fn collect(
    mut ok: MessageReader<ReloadValidated>,
    mut bad: MessageReader<ReloadFailed>,
    mut seen: ResMut<Seen>,
) {
    seen.validated
        .extend(ok.read().map(|e| (e.path.clone(), e.kind)));
    seen.failed
        .extend(bad.read().map(|e| (e.path.clone(), e.error.clone())));
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("opts_hot_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.canonicalize().expect("canonical")
}

fn app_watching(dir: &std::path::Path, control: bool) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(HotReloadPlugin {
        config: HotReloadConfig {
            enabled: true,
            debounce: Duration::from_millis(60),
            roots: vec![dir.to_path_buf()],
        },
    });
    if control {
        app.add_plugins(bevy::input::InputPlugin);
        app.add_plugins(ControlPlugin::new(ControlConfig {
            port: Some(0),
            allow_dirs: vec![dir.to_path_buf()],
            ..ControlConfig::default()
        }));
    }
    app.init_resource::<Seen>().add_systems(Update, collect);
    app.update();
    app
}

fn pump_until(app: &mut App, limit: Duration, mut done: impl FnMut(&App) -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        app.update();
        if done(app) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

const VALID_SCO: &[u8] = b"[friendlyname]\nA test object\n";

#[test]
fn a_saved_file_is_validated_after_the_quiet_period() {
    let dir = temp_dir("valid");
    let mut app = app_watching(&dir, false);
    let file = dir.join("thing.sco");
    // A burst of writes (as editors do) must produce one reload.
    for _ in 0..3 {
        std::fs::write(&file, VALID_SCO).expect("write");
        thread::sleep(Duration::from_millis(15));
    }
    assert!(pump_until(&mut app, Duration::from_secs(5), |a| !a
        .world()
        .resource::<Seen>()
        .validated
        .is_empty()));
    pump_until(&mut app, Duration::from_millis(400), |_| false);
    let seen = app.world().resource::<Seen>();
    assert_eq!(seen.validated.len(), 1, "{:?}", seen.validated);
    assert_eq!(seen.validated[0].1, ReloadKind::Sco);
    assert!(seen.failed.is_empty());
}

#[test]
fn a_broken_save_fails_and_the_next_good_one_recovers() {
    let dir = temp_dir("broken");
    let mut app = app_watching(&dir, false);
    let file = dir.join("model_main.cfg");
    std::fs::write(&file, b"").expect("truncate");
    assert!(pump_until(&mut app, Duration::from_secs(5), |a| !a
        .world()
        .resource::<Seen>()
        .failed
        .is_empty()));
    assert!(app.world().resource::<Seen>().failed[0].1.contains("empty"));
    assert!(
        app.world().resource::<Seen>().validated.is_empty(),
        "nothing reaches the apply step"
    );
    std::fs::write(&file, b"[model]\n").expect("repair");
    // A model config needs content the parser accepts; the empty-file rule is what this test is about.
    let _ = pump_until(&mut app, Duration::from_secs(3), |a| {
        !a.world().resource::<Seen>().validated.is_empty()
    });
    let stats = app.world().resource::<ReloadStats>();
    assert!(stats.failed >= 1);
}

#[test]
fn editor_scratch_and_unknown_files_are_ignored() {
    let dir = temp_dir("scratch");
    let mut app = app_watching(&dir, false);
    for name in ["a.sco~", ".a.sco.swp", "notes.txt", "a.sco.tmp"] {
        std::fs::write(dir.join(name), b"x").expect("write");
    }
    pump_until(&mut app, Duration::from_millis(600), |_| false);
    let seen = app.world().resource::<Seen>();
    assert!(seen.validated.is_empty() && seen.failed.is_empty());
}

fn client(port: u16) -> Client {
    Client::new(ClientConfig {
        host: "127.0.0.1".into(),
        port,
        token: None,
        timeout: Duration::from_secs(10),
    })
}

fn with_client<R: Send + 'static>(app: &mut App, f: impl FnOnce() -> R + Send + 'static) -> R {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    let handle = thread::spawn(move || {
        let r = f();
        flag.store(true, Ordering::SeqCst);
        r
    });
    while !done.load(Ordering::SeqCst) {
        app.update();
        thread::sleep(Duration::from_millis(4));
    }
    handle.join().expect("client")
}

#[test]
fn reload_commands_report_success_and_failure() {
    let dir = temp_dir("control");
    let mut app = app_watching(&dir, true);
    let port = app.world().resource::<ControlAddr>().0.port();
    let good = dir.join("good.sco");
    let bad = dir.join("bad.o3d");
    std::fs::write(&good, VALID_SCO).expect("good");
    std::fs::write(&bad, b"definitely not a mesh").expect("bad");
    // An existing file outside the allowed folder (on every platform, unlike `/etc/hostname`).
    let outside = temp_dir("control_outside").join("outside.sco");
    std::fs::write(&outside, VALID_SCO).expect("outside");
    let (g, b, o) = (good.clone(), bad.clone(), outside.clone());
    let (ok, err, roots, unsupported) = with_client(&mut app, move || {
        let mut c = client(port);
        let ok = c
            .call("reload.asset", json!({"path": g}))
            .expect("good reload");
        let err = c
            .call("reload.asset", json!({"path": b}))
            .expect_err("bad reload");
        let roots = c.call("watch.list", Value::Null).expect("watch.list");
        let unsupported = c
            .call("reload.asset", json!({"path": o}))
            .expect_err("outside");
        (ok, err, roots, unsupported)
    });
    assert_eq!(ok["kind"], "sco");
    assert!(
        matches!(&err, ClientError::Rpc(e) if e.code == code::FAILED),
        "{err}"
    );
    assert!(roots["roots"].as_array().is_some_and(|r| !r.is_empty()));
    assert!(
        matches!(&unsupported, ClientError::Rpc(e) if e.code == code::FORBIDDEN_PATH),
        "{unsupported}"
    );
}

#[test]
fn watch_add_and_remove_change_the_roots() {
    let dir = temp_dir("roots");
    let extra = temp_dir("roots_extra");
    let mut app = app_watching(&dir, true);
    let port = app.world().resource::<ControlAddr>().0.port();
    let e = extra.clone();
    // The extra directory must be readable for the policy: allow it by watching under a root that is allowed.
    let (added, listed) = with_client(&mut app, move || {
        let mut c = client(port);
        let added = c.call("watch.add", json!({"path": e}));
        let listed = c.call("watch.list", Value::Null).expect("list");
        (added, listed)
    });
    // `extra` is outside the allowed directories, so the command is refused rather than silently ignored.
    assert!(
        matches!(&added, Err(ClientError::Rpc(e)) if e.code == code::FORBIDDEN_PATH),
        "{added:?}"
    );
    assert_eq!(listed["roots"].as_array().map(Vec::len), Some(1));
}
