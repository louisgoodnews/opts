//! `ui.tree`, `ui.click`, `ui.focus`, `ui.action`, `ui.state` against a headless app with the GUI
//! plugin: the UI is driven by ids, exactly as a user would use it.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use bevy::prelude::*;
use opts_bevy::control::{ControlAddr, ControlConfig, ControlPlugin};
use opts_cli::client::{Client, ClientConfig, ClientError};
use opts_cli::protocol::code;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::interaction::state::{Disabled, DisabledReason};
use opts_gui::ui_id::UiId;
use opts_gui::OptsGuiPlugin;
use serde_json::{json, Value};

fn app() -> (App, u16, Arc<AtomicUsize>) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, OptsGuiPlugin));
    app.add_plugins(ControlPlugin::new(ControlConfig {
        port: Some(0),
        ..ControlConfig::default()
    }));
    let clicks = Arc::new(AtomicUsize::new(0));
    let c = clicks.clone();
    let (start, quit, load) = {
        let mut commands = app.world_mut().commands();
        let root = commands.spawn(Node::default()).id();
        let start = ButtonBuilder::filled("Start")
            .on_click(move |_, _| {
                c.fetch_add(1, Ordering::SeqCst);
            })
            .spawn(&mut commands);
        let quit = ButtonBuilder::outlined("Quit").spawn(&mut commands);
        let load = ButtonBuilder::filled("Load").spawn(&mut commands);
        commands.entity(root).add_children(&[start, quit, load]);
        (start, quit, load)
    };
    app.update();
    app.world_mut()
        .entity_mut(start)
        .insert(UiId::new("menu.start"));
    app.world_mut()
        .entity_mut(quit)
        .insert(UiId::new("menu.quit"));
    app.world_mut().entity_mut(load).insert((
        UiId::new("menu.load"),
        Disabled,
        DisabledReason::new("Needs a saved game"),
    ));
    app.update();
    let port = app.world().resource::<ControlAddr>().0.port();
    (app, port, clicks)
}

fn with_client<R: Send + 'static>(
    app: &mut App,
    port: u16,
    f: impl FnOnce(&mut Client) -> R + Send + 'static,
) -> R {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    let out = Arc::new(Mutex::new(None));
    let slot = out.clone();
    let handle = thread::spawn(move || {
        let mut c = Client::new(ClientConfig {
            host: "127.0.0.1".into(),
            port,
            token: None,
            timeout: Duration::from_secs(10),
        });
        *slot.lock().expect("slot") = Some(f(&mut c));
        flag.store(true, Ordering::SeqCst);
    });
    while !done.load(Ordering::SeqCst) {
        app.update();
        thread::sleep(Duration::from_millis(4));
    }
    handle.join().expect("client thread");
    let r = out.lock().expect("slot").take().expect("result");
    r
}

fn rpc_code(e: ClientError) -> i64 {
    match e {
        ClientError::Rpc(r) => r.code,
        other => panic!("expected an rpc error, got {other}"),
    }
}

mod errors;
mod focus;
mod tree;
