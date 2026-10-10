//! Subsystem definitions and routing filters for directed log dispatch.

use std::str::FromStr;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter::Targets;

/// Description of a target subsystem and its output file.
#[derive(Debug, Clone, Copy)]
pub struct SubsystemRoute {
    /// Base file name for this subsystem (e.g. "opts-core.log").
    pub file_name: &'static str,
    /// Target module prefixes routed to this file.
    pub target_prefixes: &'static [&'static str],
}

/// Known subsystem routes configured in the OPTS architecture.
pub const SUBSYSTEM_ROUTES: &[SubsystemRoute] = &[
    SubsystemRoute {
        file_name: "opts-core.log",
        target_prefixes: &["opts_core"],
    },
    SubsystemRoute {
        file_name: "opts-sim.log",
        target_prefixes: &["opts_sim"],
    },
    SubsystemRoute {
        file_name: "opts-bevy.log",
        target_prefixes: &["opts_bevy", "wgpu", "naga", "bevy_render"],
    },
    SubsystemRoute {
        file_name: "opts-gui.log",
        target_prefixes: &["opts_gui"],
    },
    SubsystemRoute {
        file_name: "opts-game.log",
        target_prefixes: &["opts_game"],
    },
    SubsystemRoute {
        file_name: "opts-tools.log",
        target_prefixes: &["opts_tools", "opts"],
    },
];

/// Builds a [`Targets`] filter for a specific subsystem based on the default level.
pub fn build_subsystem_filter(route: &SubsystemRoute, default_level: LevelFilter) -> Targets {
    let mut targets = Targets::new();
    for prefix in route.target_prefixes {
        targets = targets.with_target(*prefix, default_level);
    }
    targets
}

/// Parses a string level into [`LevelFilter`].
pub fn parse_level_filter(s: &str) -> LevelFilter {
    LevelFilter::from_str(s.trim()).unwrap_or(LevelFilter::INFO)
}
