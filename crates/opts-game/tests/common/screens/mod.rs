//! The screens of the application on self-authored fixture content, laid out by Bevy's real UI
//! layout in a headless app (no window system, no OMSI content). Shared by the accessibility
//! matrix and the golden-screenshot tests.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState, help::HelpPlugin, home::HomePlugin, settings::GameSettings,
    shell::ShellPlugin,
};
use opts_gui::testing::layout_base;
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::AccentPreset;

use super::layout::{preferences, resize};

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
pub use setup::build;
