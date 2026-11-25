//! The first built-in widget definitions (the table is split in two by size).

use super::super::*;

pub(super) const SPEED: HudWidget = HudWidget {
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
};

pub(super) const GEAR: HudWidget = HudWidget {
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
};

pub(super) const RPM: HudWidget = HudWidget {
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
};

pub(super) const PARKING_BRAKE: HudWidget = HudWidget {
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
};

pub(super) const RAIL_DIRECTION: HudWidget = HudWidget {
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
};

pub(super) const RAIL_LEVER: HudWidget = HudWidget {
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
};
