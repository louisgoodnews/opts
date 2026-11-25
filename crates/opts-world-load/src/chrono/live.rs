//! Systems that react to a map date change while driving and report the rebuild progress.

use super::*;

/// Handles [`MapDateChanged`]: the live Chrono switch.
pub fn apply_map_date_changes(
    mut changes: MessageReader<MapDateChanged>,
    job: Option<ResMut<WorldLoadJob>>,
    stats: Res<WorldStats>,
    mut rebuild: ResMut<ChronoRebuild>,
    mut pool: ResMut<TrafficPool>,
    mut commands: Commands,
) {
    let Some(change) = changes.read().last().copied() else {
        return;
    };
    let Some(mut job) = job else { return };
    let player = stats
        .position
        .vehicle
        .map(|(p, _)| p.tile)
        .or(stats.position.camera.map(|p| p.tile));
    let outcome = job.switch_chrono(chrono_date(change.new), player);
    for u in &outcome.unloads {
        u.apply(&mut commands);
    }
    if outcome.traffic {
        super::super::traffic::reset_pool(&mut pool);
    }
    let n = u32::try_from(outcome.rebuild.len()).unwrap_or(u32::MAX);
    rebuild.total_tiles = n;
    rebuild.pending_tiles = n;
    commands.insert_resource(job.chrono_state());
}

/// Counts [`ChronoRebuild::pending_tiles`] down as the switched tiles are rebuilt.
pub fn track_chrono_rebuild(job: Option<ResMut<WorldLoadJob>>, mut rebuild: ResMut<ChronoRebuild>) {
    if !rebuild.is_pending() {
        return;
    }
    let pending = job.map_or(0, |mut j| j.chrono_pending());
    let pending = u32::try_from(pending).unwrap_or(u32::MAX);
    if pending != rebuild.pending_tiles {
        rebuild.pending_tiles = pending;
    }
}
