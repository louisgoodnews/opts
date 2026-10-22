//! The HUD vocabulary: slots, vehicle kinds and the [`HudWidget`] descriptors.
//!
//! A widget is *data*: an id, a binding to a simulation value, a slot, a priority and a
//! visibility condition (`docs/src/roadmap/phase-9/expandability.md` section 3). The systems of
//! the HUD know the bindings, never the individual widgets; a new widget is a descriptor here
//! (or, for a binding that exists, only a profile entry).

use opts_bevy::CameraMode;
use serde::{Deserialize, Serialize};

/// A fixed region of the screen a widget lives in.
///
/// Profiles written before M-0071 may still say `Instruments`, `Hints` or `Alerts`: all three are
/// the bottom-left cluster now (the rows inside it have a fixed order, see [`Row`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Slot {
    /// Top, fades after a few seconds.
    StatusBar,
    /// Below the status bar: the camera or mode name for 1.5 s.
    CameraLabel,
    /// Bottom left: control hints, alerts and the instruments (speed, gear, parking brake) in a
    /// fixed vertical order.
    #[serde(alias = "Instruments", alias = "Hints", alias = "Alerts")]
    ClusterBottomLeft,
    /// Bottom right: the mini map.
    MiniMap,
}

impl Slot {
    /// All slots, in build order.
    pub const ALL: [Self; 4] = [
        Self::StatusBar,
        Self::CameraLabel,
        Self::ClusterBottomLeft,
        Self::MiniMap,
    ];

    /// Stable id (`ui.tree`, logs, the generated reference).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::StatusBar => "status-bar",
            Self::CameraLabel => "camera-label",
            Self::ClusterBottomLeft => "cluster-bottom-left",
            Self::MiniMap => "minimap",
        }
    }
}

/// The rows of the bottom-left cluster, top to bottom. The order is fixed: a profile cannot
/// reorder rows (it orders the widgets inside the instrument row), so the driver finds the same
/// thing in the same place in every vehicle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Row {
    /// Control hints (fade after a while).
    Hints,
    /// The alert line (a fixed minimum height, nothing moves when an alert comes and goes).
    Alerts,
    /// Speed, gear, engine speed and parking brake.
    Instruments,
}

impl Row {
    /// All rows, top to bottom.
    pub const ALL: [Self; 3] = [Self::Hints, Self::Alerts, Self::Instruments];

    /// Stable id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Hints => "hints",
            Self::Alerts => "alerts",
            Self::Instruments => "instruments",
        }
    }
}

/// The kinds of vehicle a profile exists for.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub enum VehicleKind {
    /// A single-body bus.
    #[default]
    Bus,
    /// A bus with one or more articulated sections.
    ArticulatedBus,
    /// A tram on rails.
    Tram,
    /// A trolleybus (shares the bus controls; has its own profile).
    Trolleybus,
}

impl VehicleKind {
    /// All kinds.
    pub const ALL: [Self; 4] = [
        Self::Bus,
        Self::ArticulatedBus,
        Self::Tram,
        Self::Trolleybus,
    ];

    /// Stable id (the profile file name).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Bus => "bus",
            Self::ArticulatedBus => "articulated-bus",
            Self::Tram => "tram",
            Self::Trolleybus => "trolleybus",
        }
    }
}

/// The simulation value or text source a widget shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Binding {
    /// Road speed.
    Speed,
    /// Transmission gear.
    Gear,
    /// Engine speed; only when the simulation provides it.
    Rpm,
    /// Parking brake state.
    ParkingBrake,
    /// A rail vehicle's selected direction of travel.
    RailDirection,
    /// A rail vehicle's power lever.
    RailLever,
    /// Map and entry point of the session.
    SessionInfo,
    /// Session clock and weather.
    SessionClock,
    /// The short list of the most used keys.
    ControlsHint,
    /// The alert line (the alert queue).
    Alerts,
    /// The camera label (set on camera or mode changes).
    CameraLabel,
    /// The mini map (a top-down drawing of the roads around the player).
    MiniMap,
}

impl Binding {
    /// The row of the bottom-left cluster the binding is drawn in, if it belongs to the cluster.
    #[must_use]
    pub const fn row(self) -> Option<Row> {
        match self {
            Self::Speed
            | Self::Gear
            | Self::Rpm
            | Self::ParkingBrake
            | Self::RailDirection
            | Self::RailLever => Some(Row::Instruments),
            Self::ControlsHint => Some(Row::Hints),
            Self::Alerts => Some(Row::Alerts),
            Self::SessionInfo | Self::SessionClock | Self::CameraLabel | Self::MiniMap => None,
        }
    }
}

/// How a bound value is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// A number with `precision` decimals, tabular, in the unit chosen in the settings.
    Number {
        /// Digits after the decimal point.
        precision: u8,
    },
    /// A short word or text.
    Text,
    /// A lamp: a word in the state colour (`ok`, `alarm`).
    Lamp,
    /// A drawing (the mini map).
    Graphic,
}

/// The switch that turns a widget on and off in *Settings → Interface*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetSwitch {
    /// Settings key (`hud.speed`).
    pub key: &'static str,
    /// Message key of the label.
    pub label_key: &'static str,
    /// Message key of the description.
    pub description_key: &'static str,
    /// Value when the user never chose.
    pub default_on: bool,
}

/// When a widget is visible besides its switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Condition {
    /// Vehicle kinds it applies to; empty means all.
    pub kinds: &'static [VehicleKind],
    /// Only when the simulation provides an engine speed.
    pub needs_rpm: bool,
    /// Camera modes in which it is hidden (hidden, not removed: nothing moves).
    pub hidden_in: &'static [CameraMode],
}

impl Condition {
    /// No condition.
    pub const ALWAYS: Self = Self {
        kinds: &[],
        needs_rpm: false,
        hidden_in: &[],
    };
}

/// One widget of the registry.
#[derive(Debug, Clone, Copy)]
pub struct HudWidget {
    /// Stable id, used by profiles, tests and `ui.tree` (`hud.<id>`).
    pub id: &'static str,
    /// Message key of the name.
    pub title_key: &'static str,
    /// What it shows.
    pub binding: Binding,
    /// How it writes the value.
    pub format: Format,
    /// The slot it is built into unless a profile moves it to one of `slots`.
    pub slot: Slot,
    /// Slots a profile may assign it to (the default first).
    pub slots: &'static [Slot],
    /// Higher wins when a slot has room for fewer widgets than assigned (compact windows).
    pub priority: u8,
    /// The user's on/off switch, if there is one.
    pub switch: Option<WidgetSwitch>,
    /// Further conditions.
    pub when: Condition,
}

/// Every built-in widget.
pub const WIDGETS: &[HudWidget] = &[
    HudWidget {
        id: "speed",
        title_key: "hud-widget-speed",
        binding: Binding::Speed,
        format: Format::Number { precision: 0 },
        slot: Slot::ClusterBottomLeft,
        slots: &[Slot::ClusterBottomLeft],
        priority: 100,
        switch: Some(WidgetSwitch {
            key: "hud.speed",
            label_key: "set-hud-speed",
            description_key: "set-hud-speed-desc",
            default_on: true,
        }),
        when: Condition::ALWAYS,
    },
    HudWidget {
        id: "gear",
        title_key: "hud-widget-gear",
        binding: Binding::Gear,
        format: Format::Text,
        slot: Slot::ClusterBottomLeft,
        slots: &[Slot::ClusterBottomLeft],
        priority: 80,
        switch: Some(WidgetSwitch {
            key: "hud.gear",
            label_key: "set-hud-gear",
            description_key: "set-hud-gear-desc",
            default_on: true,
        }),
        when: Condition {
            kinds: &[
                VehicleKind::Bus,
                VehicleKind::ArticulatedBus,
                VehicleKind::Trolleybus,
            ],
            needs_rpm: false,
            hidden_in: &[],
        },
    },
    HudWidget {
        id: "rpm",
        title_key: "hud-widget-rpm",
        binding: Binding::Rpm,
        format: Format::Number { precision: 0 },
        slot: Slot::ClusterBottomLeft,
        slots: &[Slot::ClusterBottomLeft],
        priority: 60,
        switch: Some(WidgetSwitch {
            key: "hud.rpm",
            label_key: "set-hud-rpm",
            description_key: "set-hud-rpm-desc",
            default_on: true,
        }),
        when: Condition {
            kinds: &[],
            needs_rpm: true,
            hidden_in: &[],
        },
    },
    HudWidget {
        id: "parking-brake",
        title_key: "hud-widget-parking-brake",
        binding: Binding::ParkingBrake,
        format: Format::Lamp,
        slot: Slot::ClusterBottomLeft,
        slots: &[Slot::ClusterBottomLeft],
        priority: 90,
        switch: Some(WidgetSwitch {
            key: "hud.parking_brake",
            label_key: "set-hud-parking-brake",
            description_key: "set-hud-parking-brake-desc",
            default_on: true,
        }),
        when: Condition {
            kinds: &[
                VehicleKind::Bus,
                VehicleKind::ArticulatedBus,
                VehicleKind::Trolleybus,
            ],
            needs_rpm: false,
            hidden_in: &[],
        },
    },
    HudWidget {
        id: "rail-direction",
        title_key: "hud-widget-rail-direction",
        binding: Binding::RailDirection,
        format: Format::Text,
        slot: Slot::ClusterBottomLeft,
        slots: &[Slot::ClusterBottomLeft],
        priority: 85,
        switch: Some(WidgetSwitch {
            key: "hud.rail_direction",
            label_key: "set-hud-rail-direction",
            description_key: "set-hud-rail-direction-desc",
            default_on: true,
        }),
        when: Condition {
            kinds: &[VehicleKind::Tram],
            needs_rpm: false,
            hidden_in: &[],
        },
    },
    HudWidget {
        id: "rail-lever",
        title_key: "hud-widget-rail-lever",
        binding: Binding::RailLever,
        format: Format::Text,
        slot: Slot::ClusterBottomLeft,
        slots: &[Slot::ClusterBottomLeft],
        priority: 90,
        switch: Some(WidgetSwitch {
            key: "hud.rail_lever",
            label_key: "set-hud-rail-lever",
            description_key: "set-hud-rail-lever-desc",
            default_on: true,
        }),
        when: Condition {
            kinds: &[VehicleKind::Tram],
            needs_rpm: false,
            hidden_in: &[],
        },
    },
    HudWidget {
        id: "session",
        title_key: "hud-widget-session",
        binding: Binding::SessionInfo,
        format: Format::Text,
        slot: Slot::StatusBar,
        slots: &[Slot::StatusBar],
        priority: 40,
        switch: Some(WidgetSwitch {
            key: "hud.session_info",
            label_key: "set-hud-session",
            description_key: "set-hud-session-desc",
            default_on: true,
        }),
        when: Condition::ALWAYS,
    },
    HudWidget {
        id: "clock",
        title_key: "hud-widget-clock",
        binding: Binding::SessionClock,
        format: Format::Text,
        slot: Slot::StatusBar,
        slots: &[Slot::StatusBar],
        priority: 50,
        switch: Some(WidgetSwitch {
            key: "hud.clock",
            label_key: "set-hud-clock",
            description_key: "set-hud-clock-desc",
            default_on: true,
        }),
        when: Condition::ALWAYS,
    },
    HudWidget {
        id: "controls-hint",
        title_key: "hud-widget-controls-hint",
        binding: Binding::ControlsHint,
        format: Format::Text,
        slot: Slot::ClusterBottomLeft,
        slots: &[Slot::ClusterBottomLeft],
        priority: 20,
        // The existing *Show hints* setting is this widget's switch.
        switch: Some(WidgetSwitch {
            key: "interface.hud_hints",
            label_key: "set-hud-hints",
            description_key: "set-hud-hints-desc",
            default_on: true,
        }),
        when: Condition::ALWAYS,
    },
    HudWidget {
        id: "alerts",
        title_key: "hud-widget-alerts",
        binding: Binding::Alerts,
        format: Format::Text,
        slot: Slot::ClusterBottomLeft,
        slots: &[Slot::ClusterBottomLeft],
        priority: 255,
        // Alerts keep the driver safe: there is no switch.
        switch: None,
        when: Condition::ALWAYS,
    },
    HudWidget {
        id: "camera-label",
        title_key: "hud-widget-camera-label",
        binding: Binding::CameraLabel,
        format: Format::Text,
        slot: Slot::CameraLabel,
        slots: &[Slot::CameraLabel],
        priority: 30,
        switch: Some(WidgetSwitch {
            key: "hud.camera_label",
            label_key: "set-hud-camera-label",
            description_key: "set-hud-camera-label-desc",
            default_on: true,
        }),
        when: Condition::ALWAYS,
    },
    HudWidget {
        id: "minimap",
        title_key: "hud-widget-minimap",
        binding: Binding::MiniMap,
        format: Format::Graphic,
        slot: Slot::MiniMap,
        slots: &[Slot::MiniMap],
        priority: 70,
        switch: Some(WidgetSwitch {
            key: "hud.minimap",
            label_key: "set-hud-minimap",
            description_key: "set-hud-minimap-desc",
            default_on: true,
        }),
        when: Condition::ALWAYS,
    },
];

/// The widget with `id`.
#[must_use]
pub fn widget(id: &str) -> Option<&'static HudWidget> {
    WIDGETS.iter().find(|w| w.id == id)
}

/// The stored value of a widget switch, from the raw setting text.
#[must_use]
pub fn switch_on(switch: &WidgetSwitch, raw: Option<&str>) -> bool {
    match raw {
        Some("true") => true,
        Some("false") => false,
        _ => switch.default_on,
    }
}
