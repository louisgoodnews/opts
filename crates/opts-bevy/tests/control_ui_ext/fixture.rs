//! The control-protocol fixture of the UI extension tests: a list, a dialog, controls and a client.

use super::*;

mod spawn;
use spawn::{spawn_controls, spawn_list};

#[derive(Resource, Default, Clone)]
pub(super) struct Chord(Arc<AtomicBool>);

fn watch_chord(keys: Res<ButtonInput<KeyCode>>, chord: Res<Chord>) {
    if keys.pressed(KeyCode::ControlLeft) && keys.pressed(KeyCode::KeyF) {
        chord.0.store(true, Ordering::SeqCst);
    }
}

pub(super) struct Fixture {
    pub(super) app: App,
    pub(super) port: u16,
    pub(super) clicks: Arc<AtomicUsize>,
    pub(super) chord: Arc<AtomicBool>,
}

/// Gives the Weather combo box's header the id `opt.weather`.
fn tag_weather_header(app: &mut App) {
    // The drop-down is operated through its header, the part that carries the id in the tree.
    // The header of the Weather combo box (the order of a query is not part of its contract).
    let world = app.world_mut();
    let header = world
        .query::<(Entity, &ComboboxHeader)>()
        .iter(world)
        .map(|(e, h)| (e, h.0))
        .collect::<Vec<_>>()
        .into_iter()
        .find(|(_, r)| {
            world
                .get::<M3Combobox>(*r)
                .is_some_and(|c| c.options[0] == "Sunny")
        })
        .map(|(e, _)| e)
        .expect("combobox header");
    app.world_mut()
        .entity_mut(header)
        .insert(UiId::new("opt.weather"));
}

pub(super) fn fixture() -> Fixture {
    let mut app = layout_app();
    app.add_plugins(ControlPlugin::new(ControlConfig {
        port: Some(0),
        ..ControlConfig::default()
    }));
    let chord = Chord::default();
    let seen = chord.0.clone();
    app.insert_resource(chord).add_systems(Update, watch_chord);
    let clicks = Arc::new(AtomicUsize::new(0));
    let c = clicks.clone();
    spawn_controls(&mut app, &c);
    spawn_list(&mut app);
    for _ in 0..6 {
        app.update();
    }
    tag_weather_header(&mut app);
    app.update();
    let port = app.world().resource::<ControlAddr>().0.port();
    Fixture {
        app,
        port,
        clicks,
        chord: seen,
    }
}

pub(super) fn with_client<R: Send + 'static>(
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
        // A failed expectation must end the pumping loop too, or the test would hang.
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut c)));
        *slot.lock().expect("slot") = Some(r);
        flag.store(true, Ordering::SeqCst);
    });
    while !done.load(Ordering::SeqCst) {
        app.update();
        thread::sleep(Duration::from_millis(4));
    }
    handle.join().expect("client thread");
    let r = out.lock().expect("slot").take().expect("result");
    match r {
        Ok(v) => v,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

pub(super) fn rpc(e: ClientError) -> (i64, Value) {
    match e {
        ClientError::Rpc(r) => (r.code, r.data.unwrap_or(Value::Null)),
        other => panic!("expected an rpc error, got {other}"),
    }
}

pub(super) fn settle(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}
