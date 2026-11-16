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

mod control;
mod files;
