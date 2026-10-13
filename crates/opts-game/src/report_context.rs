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
    hardware: Option<Res<crate::hardware::HardwareState>>,
    registry: Option<Res<crate::settings_registry::SettingsRegistry>>,
    settings: Option<Res<crate::settings::GameSettings>>,
) {
    // The profile joins a report only while the switch in Settings > Advanced is on (default off).
    let allowed = registry
        .zip(settings)
        .and_then(|(r, s)| r.text(&s, crate::hardware::ALLOW_REPORTS_KEY))
        .is_some_and(|t| t == "true");
    let hardware_text = hardware
        .as_deref()
        .and_then(|h| h.profile.as_ref())
        .filter(|_| allowed)
        .map(opts_hardware::report::render_text);
    let new = ReportContext {
        version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        git: option_env!("OPTS_GIT_HASH").map(str::to_owned),
        map: stats.as_ref().and_then(|s| non_empty(&s.map_name)),
        camera: stats
            .as_ref()
            .and(camera)
            .map(|c| format!("{} {}", c.mode.key_number(), c.mode.label())),
        vehicle: stats.as_ref().and_then(|s| non_empty(&s.vehicle_name)),
        hardware: hardware_text,
    };
    if ctx.version != new.version
        || ctx.git != new.git
        || ctx.map != new.map
        || ctx.camera != new.camera
        || ctx.vehicle != new.vehicle
        || ctx.hardware != new.hardware
    {
        *ctx = new;
    }
}
