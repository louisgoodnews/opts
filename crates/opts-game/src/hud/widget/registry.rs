//! Built-in [`HudWidget`] registry table (`WIDGETS`) and lookup helpers.

use super::*;

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
