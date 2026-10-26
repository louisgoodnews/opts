//! Search and the entry lists.

use super::*;

/// Whether a vehicle matches every word of the search text (name, id, make, model, repaint).
pub(super) fn matches_search(v: &VehicleItem, search: &str) -> bool {
    let hay = format!(
        "{} {} {} {} {}",
        v.full_name, v.id, v.manufacturer, v.model, v.default_livery
    )
    .to_lowercase();
    search
        .to_lowercase()
        .split_whitespace()
        .all(|w| hay.contains(w))
}

/// Typing into the vehicle search field (while the Select Vehicle page is up).
pub(super) fn search_input(
    mut keys: MessageReader<KeyboardInput>,
    mut menu: ResMut<VehicleMenu>,
    mut commands_in: ResMut<MenuCommands>,
) {
    // A configuration being renamed takes the keys (the name field of the Configurations page).
    if menu.page == Page::Configs && menu.configs.renaming.is_some() {
        for e in keys.read() {
            if e.state != ButtonState::Pressed {
                continue;
            }
            let Some((_, text)) = menu.configs.renaming.as_mut() else {
                break;
            };
            if e.logical_key == Key::Backspace {
                text.pop();
            } else if e.logical_key == Key::Enter {
                commands_in.0.push("commitrename".into());
            } else if let Some(t) = &e.text {
                text.extend(t.chars().filter(|c| !c.is_control()));
            }
        }
        // `menu.do typename <text>` sets the field (for scripts).
        let mut typed = None;
        commands_in
            .0
            .retain(|line| match line.strip_prefix("typename") {
                Some(rest) => {
                    typed = Some(rest.trim().to_string());
                    false
                }
                None => true,
            });
        if let (Some(t), Some((_, text))) = (typed, menu.configs.renaming.as_mut()) {
            *text = t;
        }
        return;
    }
    let on_route = menu.page == Page::Route;
    let typing = (menu.page == Page::Select && !menu.config_open) || on_route;
    let mut changed = false;
    let mut search = if on_route {
        menu.route_search.clone()
    } else {
        menu.search.clone()
    };
    for e in keys.read() {
        if !typing || e.state != ButtonState::Pressed {
            continue;
        }
        if e.logical_key == Key::Backspace {
            changed |= search.pop().is_some();
        } else if let Some(t) = &e.text {
            for ch in t.chars().filter(|c| !c.is_control()) {
                search.push(ch);
                changed = true;
            }
        }
    }
    // `menu.do search <text>` sets the field (an empty text clears it).
    let mut remote = Vec::new();
    commands_in.0.retain(|line| {
        if let Some(rest) = line.strip_prefix("search") {
            remote.push(rest.trim().to_string());
            false
        } else {
            true
        }
    });
    for text in remote {
        search = text;
        changed = true;
    }
    if on_route {
        if changed && search != menu.route_search {
            menu.route_search = search;
            menu.scroll[5] = 0.0;
        }
    } else if changed && search != menu.search {
        menu.search = search;
        menu.scroll[0] = 0.0;
    }
}

/// Entry points of the map that no AI vehicle stands on, with unique labels.
pub(super) fn free_entries(
    job: &WorldLoadJob,
    index: &EntryIndex,
    traffic: &Query<&GlobalTransform, With<TrafficVehicle>>,
) -> Vec<(u32, String)> {
    let blockers: Vec<Vec3> = traffic.iter().map(GlobalTransform::translation).collect();
    let records = job.entry_records();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut total: HashMap<&str, usize> = HashMap::new();
    for (_, n) in &records {
        *total.entry(n.as_str()).or_default() += 1;
    }
    let mut out = Vec::new();
    for (id, name) in &records {
        let n = seen.entry(name.clone()).or_default();
        *n += 1;
        let free = index.find(*id).is_none_or(|e| {
            let session = job.session_pos(e.tile, e.local);
            let p = opts_bevy::coords::omsi_to_bevy_pos(session);
            !blockers
                .iter()
                .any(|b| Vec2::new(b.x - p.x, b.z - p.z).length() < ENTRY_BLOCK_RADIUS_M)
        });
        if free {
            let label = if total.get(name.as_str()).copied().unwrap_or(1) > 1 {
                format!("{name} ({n})")
            } else {
                name.clone()
            };
            out.push((*id, label));
        }
    }
    out
}

/// The map's bus stops by name (`Busstops.cfg` of the timetables), a name that several stops
/// share numbered like the entry points.
pub(super) fn stop_list(job: &mut WorldLoadJob) -> Vec<(u32, String)> {
    let Some(tt) = job.timetables() else {
        return Vec::new();
    };
    let mut stops: Vec<(u32, String)> = tt
        .stops
        .iter()
        .filter(|s| !s.name.trim().is_empty())
        .map(|s| (s.id, s.name.trim().to_string()))
        .collect();
    stops.sort_by(|a, b| {
        a.1.to_lowercase()
            .cmp(&b.1.to_lowercase())
            .then(a.0.cmp(&b.0))
    });
    numbered(stops)
}

/// Labels that several rows share get their number: `Rathaus (1)`, `Rathaus (2)`.
fn numbered(rows: Vec<(u32, String)>) -> Vec<(u32, String)> {
    let mut total: HashMap<String, usize> = HashMap::new();
    for (_, n) in &rows {
        *total.entry(n.clone()).or_default() += 1;
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    rows.into_iter()
        .map(|(id, name)| {
            if total[&name] > 1 {
                let n = seen.entry(name.clone()).or_default();
                *n += 1;
                (id, format!("{name} ({n})"))
            } else {
                (id, name)
            }
        })
        .collect()
}

/// Marks the menu changed when the set of free entry points does.
pub(super) fn refresh_entries(
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
pub(super) fn refresh_session_rows(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stops_that_share_a_name_are_numbered() {
        let rows = vec![
            (7, "Rathaus".to_string()),
            (8, "Rathaus".to_string()),
            (3, "Markt".to_string()),
        ];
        assert_eq!(
            numbered(rows),
            vec![
                (7, "Rathaus (1)".to_string()),
                (8, "Rathaus (2)".to_string()),
                (3, "Markt".to_string()),
            ]
        );
    }
}
