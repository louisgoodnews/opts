//! Marking the menu changed when the free entries or the session vehicles change.

use super::*;

/// Marks the menu changed when the set of free entry points does.
pub(in crate::in_game::vehicle_menu) fn refresh_entries(
    mut menu: ResMut<VehicleMenu>,
    job: Option<ResMut<WorldLoadJob>>,
    index: Res<EntryIndex>,
    traffic: Query<&GlobalTransform, With<TrafficVehicle>>,
) {
    if !matches!(menu.page, Page::Select | Page::Formation) {
        return;
    }
    let Some(mut job) = job else { return };
    let total = job.entry_records().len();
    if menu.entry_total != total {
        menu.entry_total = total;
    }
    if menu.shown_stops.is_empty() {
        // The timetables are read once and cached in the job; that is no change of the job.
        let stops = stop_list(job.bypass_change_detection());
        if !stops.is_empty() {
            menu.shown_stops = stops;
        }
    }
    let free = free_entries(&job, &index, &traffic);
    let progress = (index.entries().len(), index.is_done());
    if progress != menu.index_progress {
        menu.index_progress = progress;
    }
    if free != menu.shown_entries {
        menu.shown_entries = free;
        if menu
            .entry
            .is_some_and(|e| !menu.shown_entries.iter().any(|(id, _)| *id == e))
        {
            menu.entry = None;
        }
    }
}

/// Marks the main page changed when the list of session vehicles does.
pub(in crate::in_game::vehicle_menu) fn refresh_session_rows(
    mut menu: ResMut<VehicleMenu>,
    registry: Res<SessionVehicles>,
    loc: Option<Res<opts_gui::l10n::Localization>>,
) {
    if menu.page != Page::Main {
        return;
    }
    let rows: Vec<RowSig> = registry
        .records()
        .iter()
        .filter(|r| r.is_alive())
        .map(|r| {
            let active = registry.active_id() == Some(r.id);
            let text = match &loc {
                Some(l) => crate::session_vehicles::row_text(l, r, active),
                None => r.name.clone(),
            };
            (r.id, active, text)
        })
        .collect();
    if rows != menu.shown_rows {
        menu.shown_rows = rows;
    }
}
