//! The screens of the application on self-authored fixture content, laid out by Bevy's real UI
//! layout in a headless app (no window system, no OMSI content). Shared by the accessibility
//! matrix and the golden-screenshot tests.

#![allow(dead_code)]

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};
use opts_game::{
    app_state::AppState, help::HelpPlugin, home::HomePlugin, settings::GameSettings,
    shell::ShellPlugin,
};
use opts_gui::l10n::Localization;
use opts_gui::testing::layout_base;
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::AccentPreset;

/// The user's theme choices that select `kind` and `accent` (the theme runtime turns them into
/// the `MaterialTheme`; inserting a `MaterialTheme` directly would be overwritten).
pub fn preferences(kind: ThemeKind, accent: AccentPreset) -> opts_gui::theme::ThemePreferences {
    use opts_gui::theme::{ThemeMode, ThemePreferences};
    ThemePreferences {
        mode: if kind == ThemeKind::Light {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        },
        high_contrast: kind == ThemeKind::HighContrast,
        accent,
        ..Default::default()
    }
}

/// A self-authored catalogue: invented maps and vehicles, no names or assets of any real game
/// (goldens and screenshots must never contain third-party content, ADR-0002).
pub fn fixture_catalogue() -> opts_game::new_game_screen::types::GameCatalogue {
    use opts_game::new_game_screen::types::{GameCatalogue, MapItem, VehicleItem};
    let map = |id: &str, name: &str, points: &[&str]| MapItem {
        id: id.into(),
        name: name.into(),
        friendly_name: name.into(),
        description: String::new(),
        tile_count: 4,
        standard_depot: String::new(),
        years: String::new(),
        entry_points: points.iter().map(|p| (*p).to_string()).collect(),
    };
    let mut maps = vec![
        map(
            "fixture_valley",
            "Fixture Valley",
            &["Depot", "Market square"],
        ),
        map("fixture_harbour", "Fixture Harbour", &["Pier", "Station"]),
    ];
    maps.extend((1..=8).map(|i| {
        map(
            &format!("test_hills_{i:02}"),
            &format!("Test Hills {i:02}"),
            &["Start"],
        )
    }));
    let mut vehicles = vec![
        VehicleItem::named("fixture_bus_a", "Fixture", "City Bus A"),
        VehicleItem::named("fixture_bus_b", "Fixture", "City Bus B"),
    ];
    vehicles.extend((1..=6).map(|i| {
        VehicleItem::named(
            &format!("test_coach_{i:02}"),
            "Test",
            &format!("Coach {i:02}"),
        )
    }));
    GameCatalogue { maps, vehicles }
}

/// A screen under test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Help,
    Settings,
    Wizard,
    Pause,
    FirstRunFound,
    FirstRunChoose,
    Welcome,
    Summary,
}

impl Screen {
    pub const ALL: [Screen; 9] = [
        Screen::Home,
        Screen::Help,
        Screen::Settings,
        Screen::Wizard,
        Screen::Pause,
        Screen::FirstRunFound,
        Screen::FirstRunChoose,
        Screen::Welcome,
        Screen::Summary,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Screen::Home => "home",
            Screen::Help => "help",
            Screen::Settings => "settings",
            Screen::Wizard => "wizard",
            Screen::Pause => "pause",
            Screen::FirstRunFound => "first-run-found",
            Screen::FirstRunChoose => "first-run-choose",
            Screen::Welcome => "welcome",
            Screen::Summary => "summary",
        }
    }
}

fn escape(app: &mut App) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        for _ in 0..3 {
            app.update();
        }
    }
}

/// Sets the primary window to `w` x `h` logical pixels.
pub fn resize(app: &mut App, w: u32, h: u32) {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &mut Window), With<PrimaryWindow>>();
    let mut ids = Vec::new();
    for (e, mut window) in q.iter_mut(app.world_mut()) {
        window.resolution = WindowResolution::new(w, h);
        ids.push(e);
    }
    // No window system runs headless, so the camera never learns its target size and every
    // percentage-sized root (the shell frame) would be 0 x 0: tell it directly.
    let mut cameras = app.world_mut().query::<&mut Camera>();
    for mut camera in cameras.iter_mut(app.world_mut()) {
        camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
            physical_size: UVec2::new(w, h),
            scale_factor: 1.0,
        });
    }
    for _ in 0..3 {
        app.update();
    }
}

/// Builds `screen` at `w` x `h` with `theme`.
pub fn build(screen: Screen, w: u32, h: u32, kind: ThemeKind, accent: AccentPreset) -> App {
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
    resize(&mut app, w, h);
    app.update();
    {
        use opts_game::first_run::{FirstRun, Step};
        let step = match screen {
            Screen::FirstRunFound => Some(Step::Found(std::path::PathBuf::from("/fixture/game"))),
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
    let state = match screen {
        Screen::Home | Screen::FirstRunFound | Screen::FirstRunChoose | Screen::Welcome => {
            AppState::MainMenu
        }
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

/// Switches the language of a running app (`en`, `de`, `en-XA`).
pub fn set_locale(app: &mut App, locale: &str) {
    app.world_mut()
        .resource_mut::<Localization>()
        .set_locale(locale);
    for _ in 0..4 {
        app.update();
    }
}

/// Switches the theme of a running app.
pub fn set_theme(app: &mut App, kind: ThemeKind, accent: AccentPreset) {
    app.insert_resource(preferences(kind, accent));
    for _ in 0..4 {
        app.update();
    }
}
