//! Shared test helpers that build the headless screen app and prepare screens at given sizes.
use super::input::escape;
use super::*;

/// The headless app with every screen plugin and fixture resource, before any layout.
fn screens_base(screen: Screen, w: u32, kind: ThemeKind, accent: AccentPreset) -> App {
    opts_game::isolation::isolate_user_dirs();
    let mut app = layout_base();
    let mut settings = GameSettings::default();
    // under the isolated data directory (set by `isolate_user_dirs`), never the system temp dir
    let base =
        std::env::var_os("XDG_DATA_HOME").map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let db = base.join(format!(
        "opts-a11y-{}-{}-{w}",
        std::process::id(),
        screen.name()
    ));
    let _ = std::fs::remove_dir_all(&db);
    let _ = std::fs::create_dir_all(&db);
    settings.db_path_override = Some(db.join("settings.db"));
    app.add_plugins(bevy::state::app::StatesPlugin)
        .insert_resource(settings)
        .insert_resource(preferences(kind, accent))
        .init_state::<AppState>()
        .init_state::<opts_bevy::reporting::ReportModalState>()
        .init_resource::<opts_bevy::reporting::ReportModalData>()
        .init_resource::<opts_game::in_game::VehicleMenu>()
        .init_resource::<opts_game::in_game::types::SimulationLoadState>()
        .init_resource::<opts_game::new_game_screen::types::NewGameSetup>()
        // A fixed "today": the map step's default date is shown in screenshots.
        .insert_resource(opts_game::new_game_screen::planner::Today(Some(
            opts_bevy::CalendarDate::new(1989, 10, 9),
        )))
        .insert_resource(fixture_catalogue())
        .insert_resource(opts_game::new_game_screen::configs::ConfigLibrary::at(None))
        .insert_resource(opts_game::new_game_screen::PlannerHistory::at(Some(
            db.join("drive_history.json"),
        )))
        .insert_resource(opts_game::home::HomeDrives {
            persist: false,
            ..Default::default()
        })
        .add_plugins(ShellPlugin)
        .add_plugins(HomePlugin)
        .add_plugins(HelpPlugin)
        .add_plugins(opts_game::settings_screen::SettingsScreenPlugin)
        .add_plugins(opts_game::new_game_screen::NewGameMenuPlugin)
        .add_plugins(opts_game::pause::PausePlugin)
        .add_plugins(opts_game::session_summary::SessionSummaryPlugin)
        .add_plugins(opts_game::first_run::FirstRunPlugin);
    app.world_mut()
        .resource_mut::<opts_game::first_run::FirstRun>()
        .persist = false;
    app
}

/// Puts the app into the state (first-run step, last session) that `screen` shows.
fn prepare_screen(app: &mut App, screen: Screen) {
    use opts_game::first_run::{FirstRun, Step};
    let step = match screen {
        Screen::FirstRunChoose => Some(Step::Choose),
        Screen::Welcome => Some(Step::Welcome),
        _ => None,
    };
    if step.is_some() {
        app.world_mut().resource_mut::<FirstRun>().step = step;
    }
    if screen == Screen::Summary {
        app.insert_resource(opts_game::session_summary::LastSession::new(
            &Default::default(),
            opts_game::pause::SessionStats {
                duration_s: 90.0,
                distance_m: 800.0,
                max_speed_m_s: 9.0,
                errors_at_start: 0,
                errors_now: 2,
            },
        ));
    }
}

/// Debug aid (`DBG_NEG`): reports nodes with a negative size or a radius on a collapsed node.
fn debug_negative_sizes(app: &mut App, i: usize) {
    let mut q = app
        .world_mut()
        .query::<(Entity, &ComputedNode, &Node, Option<&Name>)>();
    for (e, c, n, name) in q.iter(app.world()) {
        if c.size().min_element() < 0.0
            || n.border_radius != BorderRadius::ZERO && c.size().min_element() <= 0.0
        {
            eprintln!(
                "frame {i}: {e:?} {name:?} size {:?} radius {:?} w {:?} h {:?}",
                c.size(),
                n.border_radius,
                n.width,
                n.height
            );
        }
    }
}

/// Builds `screen` at `w` x `h` with `theme`.
pub fn build(screen: Screen, w: u32, h: u32, kind: ThemeKind, accent: AccentPreset) -> App {
    let mut app = screens_base(screen, w, kind, accent);
    resize(&mut app, w, h);
    app.update();
    prepare_screen(&mut app, screen);
    let state = match screen {
        Screen::Home | Screen::FirstRunChoose | Screen::Welcome => AppState::MainMenu,
        Screen::Summary => AppState::MainMenu,
        Screen::Help => AppState::Help,
        Screen::Settings => AppState::Settings,
        Screen::Wizard => AppState::VehicleSelection,
        Screen::Pause => AppState::InGame,
    };
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(state);
    for i in 0..8 {
        if std::env::var_os("DBG_NEG").is_some() {
            debug_negative_sizes(&mut app, i);
        }
        app.update();
    }
    if screen == Screen::Pause {
        app.world_mut()
            .resource_mut::<opts_game::in_game::types::SimulationLoadState>()
            .is_complete = true;
        escape(&mut app);
    }
    for _ in 0..4 {
        app.update();
    }
    app
}
