//! The mini map's choices (settings, zoom steps, orientation) and the update budget.

use std::time::Duration;

use crate::settings::GameSettings;

/// Setting: the map is shown (the widget switch).
pub const SETTING_ENABLED: &str = "hud.minimap";
/// Setting: the visible radius in metres.
pub const SETTING_ZOOM: &str = "hud.minimap_zoom";
/// Setting: `north` or `heading`.
pub const SETTING_ORIENTATION: &str = "hud.minimap_orientation";

/// The zoom steps: visible radius in metres, from 100 m to 1 km.
pub const ZOOM_STEPS_M: [f32; 4] = [100.0, 200.0, 500.0, 1000.0];
/// The zoom step used when the setting is missing or unreadable.
pub const DEFAULT_ZOOM_M: f32 = 200.0;
/// The map is redrawn at most this often (10 Hz).
pub const UPDATE_INTERVAL: Duration = Duration::from_millis(100);
/// The budget of one redraw, in microseconds (the benchmark test and `hud.state` use it).
pub const BUDGET_US: u64 = 1000;

/// What points up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// North is up.
    North,
    /// The player's heading is up.
    #[default]
    Heading,
}

impl Orientation {
    /// Setting text.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::Heading => "heading",
        }
    }

    /// From the setting text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "north" => Some(Self::North),
            "heading" => Some(Self::Heading),
            _ => None,
        }
    }

    /// The other one.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::North => Self::Heading,
            Self::Heading => Self::North,
        }
    }
}

/// The user's choices, read from the settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MiniMapConfig {
    /// Shown at all.
    pub enabled: bool,
    /// Visible radius in metres.
    pub zoom_m: f32,
    /// What points up.
    pub orientation: Orientation,
}

impl Default for MiniMapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            zoom_m: DEFAULT_ZOOM_M,
            orientation: Orientation::default(),
        }
    }
}

impl MiniMapConfig {
    /// Reads the three settings (missing or unreadable values give the defaults).
    #[must_use]
    pub fn read(settings: &GameSettings) -> Self {
        let d = Self::default();
        Self {
            enabled: settings
                .raw(SETTING_ENABLED)
                .map_or(d.enabled, |v| v != "false"),
            zoom_m: settings
                .raw(SETTING_ZOOM)
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|z| ZOOM_STEPS_M.contains(z))
                .unwrap_or(d.zoom_m),
            orientation: settings
                .raw(SETTING_ORIENTATION)
                .and_then(|v| Orientation::parse(&v))
                .unwrap_or(d.orientation),
        }
    }
}

/// The zoom step next to `zoom_m`: `+1` zooms in (smaller radius), `-1` zooms out. Stays at the
/// ends.
#[must_use]
pub fn step_zoom(zoom_m: f32, direction: i32) -> f32 {
    let at = ZOOM_STEPS_M
        .iter()
        .position(|z| (*z - zoom_m).abs() < 0.5)
        .unwrap_or(1);
    let next = (at as i32 - direction).clamp(0, ZOOM_STEPS_M.len() as i32 - 1);
    ZOOM_STEPS_M[next as usize]
}
