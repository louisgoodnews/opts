//! The advanced settings: the developer panel, the logs, the scene trace and the streaming budget.

use crate::settings_registry::descriptor::{
    Category as C, SettingDescriptor as D, SettingKind as K,
};

use super::enums::{LOG_LEVELS, TRACE_MODES};

/// The developer panel, the log level and the scene trace.
pub(super) fn advanced() -> Vec<D> {
    vec![
        D::new(
            "advanced.developer_panel",
            K::Bool,
            "false",
            C::Advanced,
            10,
            "set-developer-panel",
            "set-developer-panel-desc",
        )
        .keywords(&["f10", "debug", "statistics", "overlay"])
        .since("0.2"),
        D::new(
            "advanced.log_level",
            K::Enum(LOG_LEVELS),
            "info",
            C::Advanced,
            20,
            "set-log-level",
            "set-log-level-desc",
        )
        .keywords(&["logging", "diagnostics", "debug"])
        .since("0.2"),
        D::new(
            "advanced.trace_scene",
            K::Enum(TRACE_MODES),
            "off",
            C::Advanced,
            60,
            "set-trace-scene",
            "set-trace-scene-desc",
        )
        .keywords(&[
            "developer",
            "debug",
            "mesh",
            "trace",
            "asset",
            "tile",
            "jsonl",
        ])
        .since("0.3"),
        D::new(
            "advanced.trace_overlay",
            K::Bool,
            "false",
            C::Advanced,
            70,
            "set-trace-overlay",
            "set-trace-overlay-desc",
        )
        .keywords(&["developer", "debug", "boxes", "overlay", "trace"])
        .since("0.3"),
    ]
}

/// The tile build budget while streaming.
pub(super) fn streaming() -> Vec<D> {
    vec![
        D::new(
            "advanced.streaming_budget_ms",
            K::Int {
                min: 0,
                max: 50,
                step: 1,
            },
            "8",
            C::Advanced,
            80,
            "set-streaming-budget",
            "set-streaming-budget-desc",
        )
        .keywords(&[
            "map",
            "tiles",
            "streaming",
            "stutter",
            "freeze",
            "frame",
            "performance",
        ])
        .since("0.3"),
        D::new(
            "streaming.mode",
            K::Enum(super::enums::STREAMING_MODES),
            "auto",
            C::Advanced,
            81,
            "set-streaming-mode",
            "set-streaming-mode-desc",
        )
        .keywords(&["map", "tiles", "streaming", "speed", "object", "tile", "regime"])
        .since("0.3"),
    ]
}
