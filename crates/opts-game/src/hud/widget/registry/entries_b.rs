//! The second built-in widget definitions (the table is split in two by size).

use super::super::*;

pub(super) const SESSION: HudWidget = HudWidget {
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
};

pub(super) const CLOCK: HudWidget = HudWidget {
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
};

pub(super) const CONTROLS_HINT: HudWidget = HudWidget {
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
};

pub(super) const ALERTS: HudWidget = HudWidget {
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
};

pub(super) const CAMERA_LABEL: HudWidget = HudWidget {
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
};

pub(super) const MINIMAP: HudWidget = HudWidget {
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
};

pub(super) const FUEL: HudWidget = HudWidget {
    id: "fuel",
    title_key: "hud-widget-fuel",
    binding: Binding::Fuel,
    format: Format::Number { precision: 0 },
    slot: Slot::ClusterBottomLeft,
    slots: &[Slot::ClusterBottomLeft],
    priority: 75,
    switch: None,
    when: Condition::ALWAYS,
};

pub(super) const FUEL_LOW: HudWidget = HudWidget {
    id: "fuel-low",
    title_key: "hud-widget-fuel-low",
    binding: Binding::FuelLow,
    format: Format::Lamp,
    slot: Slot::ClusterBottomLeft,
    slots: &[Slot::ClusterBottomLeft],
    priority: 70,
    switch: None,
    when: Condition::ALWAYS,
};
