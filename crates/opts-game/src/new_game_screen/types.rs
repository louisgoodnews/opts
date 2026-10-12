//! Type definitions, ECS components, and state resources for the Start New Game menu.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Marker component for the root container of the new game menu screen.
#[derive(Component)]
pub struct NewGameScreenRoot;

/// Marker component for the "Back to Main Menu" button.
#[derive(Component)]
pub struct BackToMenuBtn;

/// Marker component for the "Launch Simulation" button.
#[derive(Component)]
pub struct LaunchGameBtn;

/// Marker component for map selection buttons in the scrolled left list.
#[derive(Component, Debug, Clone)]
pub struct SelectMapBtn {
    pub map_id: String,
}

/// Marker component for entry point selection chips in the map detail panel.
#[derive(Component, Debug, Clone)]
pub struct SelectEntryPointBtn {
    pub entry_point: String,
}

/// Marker component for vehicle selection buttons in the scrolled left list.
#[derive(Component, Debug, Clone)]
pub struct SelectVehicleBtn {
    pub vehicle_id: String,
}

/// Marker component for weather preset buttons.
#[derive(Component)]
pub struct SelectWeatherBtn {
    pub preset_name: &'static str,
}

/// Marker component for adjusting simulation hour.
#[derive(Component)]
pub struct AdjustTimeBtn {
    pub hour_offset: i32,
}

/// Marker component for the departure date & time display text inside the Datetime tab.
#[derive(Component)]
pub struct DepartureTimeDisplayText;

/// Marker component for the summary status text in the footer.
#[derive(Component)]
pub struct LaunchSummaryText;

/// Kind of scrollable list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollKind {
    Maps,
    Vehicles,
    EntryPoints,
    /// Multi-line map description text.
    MapDescription,
    Repaints,
    Weather,
}

/// Marker for an outer scroll viewport container.
#[derive(Component, Debug, Clone)]
pub struct ScrollableContainer {
    pub kind: ScrollKind,
    pub scroll_y: f32,
    pub max_scroll_y: f32,
}

/// Marker for the scrollable inner content node.
#[derive(Component, Debug, Clone)]
pub struct ScrollContent {
    pub kind: ScrollKind,
}

/// Button scrolling the list upwards.
#[derive(Component, Debug, Clone)]
pub struct ScrollUpBtn {
    pub kind: ScrollKind,
}

/// Button scrolling the list downwards.
#[derive(Component, Debug, Clone)]
pub struct ScrollDownBtn {
    pub kind: ScrollKind,
}

/// Marker component for the 70% right frame showing selected map details.
#[derive(Component)]
pub struct MapDetailPanelRoot;

/// Marker component for the 70% right frame showing selected vehicle details.
#[derive(Component)]
pub struct VehicleDetailPanelRoot;

/// Item representing a discovered or catalogue OMSI map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapItem {
    pub id: String,
    pub name: String,
    pub friendly_name: String,
    pub description: String,
    pub tile_count: usize,
    pub standard_depot: String,
    pub years: String,
    pub entry_points: Vec<String>,
}

/// Item representing a playable user bus in the simulation fleet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VehicleItem {
    pub id: String,
    pub manufacturer: String,
    pub model: String,
    pub full_name: String,
    pub default_livery: String,
    /// `.bus` path relative to the OMSI root (empty when unknown).
    #[serde(default)]
    pub bus_file: String,
    pub description: String,
    pub engine_power: String,
    pub transmission: String,
    pub capacity: String,
    pub length_m: String,
    pub max_speed_kmh: String,
    pub passenger_doors: String,
}

/// Resource caching all available maps and vehicles for the menu.
#[derive(Resource, Debug, Clone, Default)]
pub struct GameCatalogue {
    pub maps: Vec<MapItem>,
    pub vehicles: Vec<VehicleItem>,
}

/// Active configuration for an upcoming new game session.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct NewGameSetup {
    pub selected_map: String,
    pub selected_entry_point: String,
    pub selected_vehicle: String,
    pub selected_livery: String,
    pub weather_preset: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub clock_mode: ClockPickerMode,
    pub year_editing: bool,
    pub year_buffer: String,
    pub setvars_modal_open: bool,
    pub custom_setvars: HashMap<String, i32>,
}

/// Active mode for the Material Clock Picker dial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClockPickerMode {
    #[default]
    Hours,
    Minutes,
}

impl Default for NewGameSetup {
    fn default() -> Self {
        Self {
            selected_map: "Berlin-Spandau".to_string(),
            selected_entry_point: "Spandau Westerwaldstr./Siegener Str.".to_string(),
            selected_vehicle: "MAN SD200 - SD77".to_string(),
            selected_livery: String::new(),
            weather_preset: "Clear Summer Day".to_string(),
            year: 1989,
            month: 10,
            day: 9,
            hour: 8,
            minute: 30,
            clock_mode: ClockPickerMode::Hours,
            year_editing: false,
            year_buffer: "1989".to_string(),
            setvars_modal_open: false,
            custom_setvars: HashMap::new(),
        }
    }
}

/// Resource caching user search queries and focus states.
#[derive(Resource, Debug, Clone, Default)]
pub struct CatalogueSearchState {
    pub map_query: String,
    pub vehicle_query: String,
}

/// Search input box entity marker.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchInputBox {
    pub kind: ScrollKind,
}

/// Marker component attached to map item card for live text search filtering.
#[derive(Component, Debug, Clone)]
pub struct MapCardItem {
    pub map_id: String,
    pub search_text: String,
}

/// Marker component attached to vehicle item card for live text search filtering.
#[derive(Component, Debug, Clone)]
pub struct VehicleCardItem {
    pub vehicle_id: String,
    pub search_text: String,
}

/// Marker for vertical scrollbar background track.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarTrack {
    pub kind: ScrollKind,
}

/// Marker for vertical scrollbar indicator thumb.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarThumb {
    pub kind: ScrollKind,
}

/// Floating action button scrolling the list to the top.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollToTopBtn {
    pub kind: ScrollKind,
}

/// Marker for the selection summary card at the bottom of notebook tabs.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabSummaryText;

/// Button switching calendar month backwards.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrevMonthBtn;

/// Button switching calendar month forwards.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextMonthBtn;

/// Button selecting a specific calendar day (1..=31).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectCalendarDayBtn {
    pub day: u32,
}

/// Header date text in Material Date Picker ("Mon, Apr 18").
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarHeaderDateText;

/// Month and year title text in Material Date Picker ("April 2020").
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarMonthYearText;

/// Button incrementing the year in the Date Picker spinbox.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearIncrementBtn;

/// Button decrementing the year in the Date Picker spinbox.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearDecrementBtn;

/// Input box container for typing the year directly in the spinbox.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearInputBox;

/// Text entity displaying the year inside the spinbox.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearInputText;

/// Button selecting a 12-hour digit on the Material Clock Picker dial.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectClockHourBtn {
    pub hour_12: u32,
}

/// Button selecting a minute step (00, 05, 10, ..., 55) on the Material Clock Picker dial.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectClockMinuteBtn {
    pub minute: u32,
}

/// Button clicking the hours digital readout in the clock header ("8:30" -> "8").
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockHourTextBtn;

/// Button clicking the minutes digital readout in the clock header ("8:30" -> "30").
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockMinuteTextBtn;

/// Container holding the radial hour buttons (1..=12).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockDialHourContainer;

/// Container holding the radial minute buttons (00, 05, ..., 55).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockDialMinuteContainer;

/// Marker for the clock hand pointer indicator line.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockHandPointer;

/// Button toggling AM / PM on the Material Clock Picker dial.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectClockMeridianBtn {
    pub is_pm: bool,
}

/// Digital time text display in Material Clock Picker header ("7:04 PM").
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockDigitalDisplayText;

/// Quick minute adjustment button on the Material Clock Picker (-15m, +15m).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdjustMinuteBtn {
    pub minute_offset: i32,
}

/// Marker component for entry point cards in the scrollable entrypoint selection.
#[derive(Component, Debug, Clone)]
pub struct EntryPointCardItem {
    pub entry_point: String,
}

/// Marker component for livery/repaint selection buttons.
#[derive(Component, Debug, Clone)]
pub struct SelectLiveryBtn {
    pub livery: String,
}

/// Marker component for repaint cards in the scrollable repaint selection.
#[derive(Component, Debug, Clone)]
pub struct RepaintCardItem {
    pub livery: String,
}

/// Button opening the bus setvars configuration modal dialog.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenSetvarsModalBtn;

/// Button closing the bus setvars configuration modal dialog.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseSetvarsModalBtn;

/// Button stepping a setvar integer value (+1 / -1).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SetvarStepBtn {
    pub var_name: String,
    pub delta: i32,
}

/// Button resetting setvars to defaults.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetSetvarsBtn;

/// Marker component for the setvars modal overlay container.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetvarsModalRoot;

/// Marker component for weather cards in the scrollable weather preset list.
#[derive(Component, Debug, Clone)]
pub struct WeatherCardItem {
    pub preset_name: String,
}

/// Marker component for the center frame displaying selected weather details.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeatherDetailPanelRoot;

/// Button requesting a refresh of live device-location weather.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshWeatherBtn;

/// Button applying live device-location weather to the simulation setup.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyLiveWeatherBtn;

/// Resource caching real-time live weather based on device location.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct DeviceLocationWeather {
    pub location_name: String,
    pub condition_desc: String,
    pub temperature_c: f32,
    pub wind_speed_kmh: f32,
    pub humidity_pct: u32,
    pub is_loaded: bool,
}

impl Default for DeviceLocationWeather {
    fn default() -> Self {
        Self {
            location_name: "Detecting Device Location...".to_string(),
            condition_desc: "Overcast with light breeze".to_string(),
            temperature_c: 12.0,
            wind_speed_kmh: 26.0,
            humidity_pct: 91,
            is_loaded: false,
        }
    }
}
