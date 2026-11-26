//! The screens of the application on self-authored fixture content, laid out by Bevy's real UI
//! layout in a headless app (no window system, no OMSI content). Shared by the accessibility
//! matrix and the golden-screenshot tests.

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
    FirstRunChoose,
    Welcome,
    Summary,
}

impl Screen {
    pub const ALL: [Screen; 8] = [
        Screen::Home,
        Screen::Help,
        Screen::Settings,
        Screen::Wizard,
        Screen::Pause,
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
            Screen::FirstRunChoose => "first-run-choose",
            Screen::Welcome => "welcome",
            Screen::Summary => "summary",
        }
    }
}

mod input;
mod setup;
pub use input::{set_locale, set_theme};
pub use setup::build;

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
