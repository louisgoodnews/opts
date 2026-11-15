//! The harness: the Bevy app with the capture rig, the generated map and the event sink.

use super::*;

/// The running Bevy app plus the benchmark state around it.
pub struct Harness {
    pub(super) apps: SubApps,
    pub(super) rig: CaptureRig,
    /// The generated map.
    pub map: MapWriter,
    /// Installation used by preflight (same lookups as the loader).
    pub install: OmsiInstall,
    /// Captured `WARN`/`ERROR` events.
    pub sink: EventSink,
    /// Events of the empty map.
    pub baseline: Baseline,
    /// Settings.
    pub config: WorkerConfig,
}

impl Harness {
    /// Builds the app, writes the empty map and loads it.
    ///
    /// # Errors
    /// The map cannot be written or loaded, or it does not settle.
    pub fn new(config: WorkerConfig, sink: EventSink) -> Result<Self> {
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                })
                .disable::<WinitPlugin>()
                .disable::<bevy::log::LogPlugin>()
                .disable::<bevy::audio::AudioPlugin>()
                .disable::<bevy::gilrs::GilrsPlugin>(),
        );
        // The scenery runtime is driven by hand (`advance_scenery`); the static views see every
        // LOD group, as before the runtime existed.
        app.add_plugins(opts_bevy::SceneryRuntimePlugin)
            .insert_resource(SceneryRuntimeSettings {
                automatic: false,
                ..default()
            })
            .insert_resource(SceneryLodMode::All)
            .insert_resource(SceneryClock::at(2026, 6, 29, 12.0 * 3600.0));
        app.finish();
        app.cleanup();
        let mut apps = std::mem::take(app.sub_apps_mut());
        let rig = CaptureRig::spawn(apps.main.world_mut());

        let map = MapWriter {
            dir: config
                .run_dir
                .join(format!("work/worker-{}/map", config.id)),
            anchor: config.anchor.clone(),
        };
        map.write_initial()?;
        let catalogue = lookups::open(config.catalogue.as_deref());
        let install = lookups::install(&config.omsi_root, catalogue.as_ref());
        let session = OmsiSessionConfig {
            root: config.omsi_root.clone(),
            map_dir: map.dir.clone(),
            vehicle_path: None,
            entry_point: super::super::mapgen::ENTRY_NAME.into(),
            start_station: None,
            livery: String::new(),
            setvars: Vec::new(),
            human_path: None,
            lookup: catalogue.as_ref().map(lookups::path_lookup),
            name_lookup: catalogue.as_ref().map(lookups::name_lookup),
            twin_lookup: catalogue.as_ref().map(lookups::twin_lookup),
            relocate_lookup: catalogue.as_ref().map(lookups::relocate_lookup),
        };
        let job = begin_world_load(session).map_err(|e| anyhow!("begin_world_load: {e}"))?;
        apps.main.world_mut().insert_resource(job);
        apps.main.world_mut().insert_resource(WorldStats::default());
        let mut h = Self {
            apps,
            rig,
            map,
            install,
            sink,
            baseline: Baseline::default(),
            config,
        };
        if !h.settle(Instant::now() + Duration::from_secs(60))? {
            return Err(anyhow!("the empty map did not settle"));
        }
        h.baseline = Baseline::from_events(&h.drain_events());
        Ok(h)
    }
}
