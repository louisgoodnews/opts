//! Feeds the F1 report with facts about the running session.

use bevy::prelude::*;
use opts_bevy::reporting::ReportContext;
use opts_bevy::CameraState;

use crate::in_game::omsi_world::WorldStats;

fn non_empty(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_owned())
}

/// Refreshes [`ReportContext`] every frame (cheap string compares only on change).
pub fn update_report_context(
    mut ctx: ResMut<ReportContext>,
    stats: Option<Res<WorldStats>>,
    camera: Option<Res<CameraState>>,
) {
    let new = ReportContext {
        version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        git: option_env!("OPTS_GIT_HASH").map(str::to_owned),
        map: stats.as_ref().and_then(|s| non_empty(&s.map_name)),
        camera: stats
            .as_ref()
            .and(camera)
            .map(|c| format!("{} {}", c.mode.key_number(), c.mode.label())),
        vehicle: stats.as_ref().and_then(|s| non_empty(&s.vehicle_name)),
    };
    if ctx.version != new.version
        || ctx.git != new.git
        || ctx.map != new.map
        || ctx.camera != new.camera
        || ctx.vehicle != new.vehicle
    {
        *ctx = new;
    }
}
