//! Bevy application composition builder for OPTS.

use bevy::log::LogPlugin;
use bevy::prelude::*;
use std::time::Duration;

use crate::settings::GameSettings;
use crate::shutdown::ShutdownPlugin;
use crate::OptsGamePlugin;
use opts_bevy::OptsAssetPlugin;
use opts_cli::CliArgs;

/// Resource storing the parsed CLI arguments.
#[derive(Resource, Debug, Clone)]
pub struct AppArgs(pub CliArgs);

/// Constructs and configures a complete Bevy [`App`] ready for execution.
///
/// This function separates application construction from running, allowing both
/// interactive execution in `main.rs` and headless testing in integration test harnesses.
pub fn build_app(args: CliArgs) -> App {
    let mut app = App::new();

    // Store CLI arguments as a resource
    app.insert_resource(AppArgs(args.clone()));

    // 1. Configure Window and Engine plugins based on headless flag
    if args.headless {
        app.add_plugins(
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                Duration::from_millis(16),
            )),
        )
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(bevy::input::InputPlugin);
    } else {
        let (width, height) = args.parse_dimensions();
        let mode = if args.fullscreen && !args.windowed {
            bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current)
        } else {
            bevy::window::WindowMode::Windowed
        };

        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "OPTS — Open Public Transport Simulator".to_string(),
                        resolution: bevy::window::WindowResolution::new(
                            width as u32,
                            height as u32,
                        ),
                        mode,
                        resize_constraints: bevy::window::WindowResizeConstraints {
                            min_width: 800.0,
                            min_height: 600.0,
                            ..default()
                        },
                        ..default()
                    }),
                    ..default()
                })
                .build()
                .disable::<LogPlugin>(),
        );
    }

    // 2. Load or update persistent user settings
    let mut settings = GameSettings::load_or_default();
    if let Some(ref path) = args.map_path {
        settings.recent_map_path = Some(path.clone());
    }
    if let Some(ref path) = args.recent_map {
        settings.recent_map_path = Some(path.clone());
        let _ = settings.save();
    }
    app.insert_resource(settings);

    // Remote control (opt-in: `--cli-port`).
    if args.cli_port.is_some() {
        app.add_plugins(opts_bevy::control::ControlPlugin::new(
            opts_bevy::control::ControlConfig {
                port: args.cli_port,
                bind: args.cli_bind.clone(),
                token: args.cli_token.clone(),
                allow_dirs: args.cli_allow_dir.clone(),
            },
        ));
    }

    // Hot reload of edited OMSI files (on by default; `--no-hot-reload` disables the watcher).
    app.add_plugins(opts_bevy::hot_reload::HotReloadPlugin {
        config: opts_bevy::hot_reload::HotReloadConfig {
            enabled: !args.no_hot_reload,
            debounce: Duration::from_millis(args.hot_reload_debounce_ms),
            roots: args.hot_reload_root.clone(),
        },
    })
    .add_plugins(crate::in_game::hot_reload::GameHotReloadPlugin);

    // 3. Register engine bridges, UI, and application plugins
    app.add_plugins(OptsAssetPlugin)
        .add_plugins(OptsGamePlugin)
        .add_plugins(crate::catalogue_service::CataloguePlugin)
        .add_plugins(ShutdownPlugin)
        .add_plugins(crate::control::GameControlPlugin);

    // F1 bug reports (screenshot + text); needs a window, so not in headless runs.
    if !args.headless {
        app.add_plugins(opts_bevy::ground_material::GroundMaterialPlugin)
            .add_plugins(opts_bevy::reporting::ReportingPlugin)
            .add_systems(Update, crate::report_context::update_report_context);
    }

    app
}
