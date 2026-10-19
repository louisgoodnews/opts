//! The control-protocol fixture of the UI extension tests: a list, a dialog, controls and a client.

use super::*;

#[derive(Resource, Default, Clone)]
pub(super) struct Chord(Arc<AtomicBool>);

fn watch_chord(keys: Res<ButtonInput<KeyCode>>, chord: Res<Chord>) {
    if keys.pressed(KeyCode::ControlLeft) && keys.pressed(KeyCode::KeyF) {
        chord.0.store(true, Ordering::SeqCst);
    }
}

/// A windowed list of 300 entries beside the other fixtures (the wizard's map list is one).
fn spawn_list(app: &mut App) -> Entity {
    use opts_gui::l10n::{wizard_keys::md, Localization};
    use opts_gui::patterns::list_model::ListEntry;
    use opts_gui::patterns::master_detail::{
        spawn_master_detail, MasterDetail, MasterDetailConfig,
    };
    let loc = app.world().resource::<Localization>().clone();
    let entries: Vec<ListEntry> = (0..300)
        .map(|i| ListEntry::new(format!("id{i}"), format!("Entry {i:05}")))
        .collect();
    let mut commands = app.world_mut().commands();
    let parts = spawn_master_detail(
        &mut commands,
        &loc,
        MasterDetailConfig {
            prefix: "maps",
            empty_key: md::NO_RESULTS,
            search_key: md::SEARCH,
        },
        MasterDetail::new(entries, None),
    );
    let frame = commands
        .spawn(Node {
            width: Val::Px(900.0),
            height: Val::Px(500.0),
            ..default()
        })
        .id();
    commands.entity(frame).add_child(parts.root);
    parts.root
}

pub(super) struct Fixture {
    pub(super) app: App,
    pub(super) port: u16,
    pub(super) clicks: Arc<AtomicUsize>,
    pub(super) chord: Arc<AtomicBool>,
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
    {
        let mut commands = app.world_mut().commands();
        // A 300 x 200 scroll view with 30 buttons of about 40 px.
        let view = commands
            .spawn((
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(200.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                ScrollPosition::default(),
                UiId::new("list.view"),
            ))
            .id();
        let rows: Vec<Entity> = (0..30)
            .map(|i| {
                let mut b = ButtonBuilder::filled(format!("Row {i}"));
                if i == 0 {
                    let c = c.clone();
                    b = b.on_click(move |_, _| {
                        c.fetch_add(1, Ordering::SeqCst);
                    });
                }
                let e = b.spawn(&mut commands);
                commands.entity(e).insert(UiId::new(format!("row.{i}")));
                e
            })
            .collect();
        commands.entity(view).add_children(&rows);
        let sw = SwitchBuilder::standalone().spawn(&mut commands);
        commands.entity(sw).insert(UiId::new("opt.switch"));
        let combo = ComboboxBuilder::new("Weather")
            .with_options(vec!["Sunny".into(), "Rain".into(), "Fog".into()])
            .with_selected(0)
            .spawn(&mut commands);
        let _ = combo;
        let long = ComboboxBuilder::new("Start")
            .with_options((0..12).map(|i| format!("Start point {i}")).collect())
            .spawn(&mut commands);
        commands.entity(long).insert(UiId::new("opt.start"));
        let field = TextFieldBuilder::outlined("Search").spawn(&mut commands);
        commands.entity(field).insert(UiId::new("opt.search"));
    }
    spawn_list(&mut app);
    for _ in 0..6 {
        app.update();
    }
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
