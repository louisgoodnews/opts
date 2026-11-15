//! The developer panel's text: the short page, the long details page and the vehicle speed.

use bevy::prelude::*;

use super::super::WorldStats;
use super::position::format_pos;

/// The long page (Shift+F10): load counters, the camera and vehicle positions and asset problems.
fn details_text(stats: &WorldStats) -> String {
    format!(
        "[F10] map: {} (start tile {:?})\n{}\n{}\ntiles loaded: {} (missing {})\nscenery objects: {} (trees {}, protected-skipped {}, failed {}, no mesh {}) | splines: {} (failed {})\nvehicle: {} ({} meshes, {} verts)\nscene vertices: {} | asset failures: {} | load problems: {}\nkeyed meshes decoded exactly: {}",
        stats.map_name,
        stats.start_tile,
        format_pos("camera ", stats.position.camera, &stats.grid),
        stats.position.vehicle.map_or_else(
            || "vehicle: N/A".to_string(),
            |(p, h)| format!("{} heading {h:.0}°", format_pos("vehicle", Some(p), &stats.grid)),
        ),
        stats.tiles_loaded,
        stats.tiles_missing,
        stats.objects_loaded,
        stats.trees_loaded,
        stats.objects_protected,
        stats.objects_failed,
        stats.objects_no_geometry,
        stats.splines_loaded,
        stats.splines_failed,
        if stats.vehicle_name.is_empty() { "N/A" } else { &stats.vehicle_name },
        stats.vehicle_meshes,
        stats.vehicle_vertices,
        stats.scene_vertices,
        stats.asset_failures,
        stats.load_problems,
        stats.meshes_exact,
    )
}

/// The optional session facts the developer panel shows besides the world statistics.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PanelSources<'w, 's> {
    control: Option<Res<'w, opts_bevy::control::state::ControlState>>,
    environment: Option<Res<'w, opts_bevy::SessionEnvironment>>,
    traffic: Option<Res<'w, super::super::traffic::TrafficState>>,
    scheduled: Option<Res<'w, super::super::traffic::ScheduledTraffic>>,
    stops: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            &'static opts_bevy::bus_stops::BusStopMarker,
        ),
    >,
}

/// Refreshes the developer panel text while it is visible.
pub fn update_debug_overlay(
    stats: Res<WorldStats>,
    detail: Res<super::super::overlay_info::DeveloperDetail>,
    vehicle: Query<
        (&opts_bevy::VehicleController, &GlobalTransform),
        With<opts_bevy::PlayerVehicle>,
    >,
    sources: PanelSources,
    mut query: Query<(&mut Text, &mut Visibility), With<super::DebugOverlayText>>,
) {
    let PanelSources {
        control,
        environment,
        traffic,
        scheduled,
        stops,
    } = sources;
    for (mut text, mut vis) in &mut query {
        *vis = if stats.overlay_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !stats.overlay_visible {
            continue;
        }
        let facts = super::super::overlay_info::PanelFacts {
            fps: control.as_ref().map(|c| {
                let worst = c.frame_log.iter().map(|(_, ms)| *ms).fold(0.0, f32::max);
                (c.fps, worst)
            }),
            clock: environment.as_ref().map(|e| e.clock_text()),
            vehicle: stats.position.vehicle.map(|(p, h)| {
                let abs = stats.grid.absolute(p.tile, p.local.as_dvec2());
                ([abs.x, abs.y], p.world.z, h, speed_kmh(&vehicle))
            }),
            session: vehicle
                .single()
                .ok()
                .map(|(_, t)| [t.translation().x, -t.translation().z]),
            traffic: traffic.as_deref(),
            scheduled: scheduled.as_deref(),
            stops: stops
                .iter()
                .map(|(t, m)| (m.id_code, [t.translation().x, -t.translation().z]))
                .collect(),
        };
        text.0 = format!(
            "[F10 off, Shift+F10 {}] map: {}\n{}",
            if detail.full { "short page" } else { "details" },
            stats.map_name,
            super::super::overlay_info::lines(&facts)
        );
        text.0.push('\n');
        text.0
            .push_str(&format_pos("camera ", stats.position.camera, &stats.grid));
        if let Some((p, _)) = stats.position.vehicle {
            text.0.push('\n');
            text.0
                .push_str(&format_pos("vehicle", Some(p), &stats.grid));
        }
        if !detail.full {
            continue;
        }
        text.0.push('\n');
        text.0.push_str(&details_text(&stats));
        if let Ok((c, _)) = vehicle.single() {
            text.0.push('\n');
            text.0
                .push_str(&crate::control_dynamics::dynamics_text(&c.sim));
        }
    }
}

/// The player's vehicle speed (km/h), 0 without one.
fn speed_kmh(
    vehicle: &Query<
        (&opts_bevy::VehicleController, &GlobalTransform),
        With<opts_bevy::PlayerVehicle>,
    >,
) -> f32 {
    vehicle
        .single()
        .map_or(0.0, |(c, _)| c.sim.chassis.forward_speed_km_h().abs())
}
