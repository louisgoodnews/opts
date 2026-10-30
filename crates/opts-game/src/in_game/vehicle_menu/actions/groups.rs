//! Handlers of the vehicle, navigation, route and change-of-map action groups.

use super::super::route_page::{clear_route, jump_to_entry, pick_route, pick_route_text, pick_run};
use super::*;

/// Despawn, switch, respawn and reset of session vehicles. Returns the action when this group does not handle it.
pub(super) fn vehicle_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::Despawn => {
            match c.registry.active_id() {
                None => c.menu.status = "There is no vehicle to despawn.".into(),
                Some(id) => {
                    crate::session_vehicles::defer(c.commands, move |w: &mut World| {
                        if let Err(e) = ops::despawn(w, id, DespawnReason::User) {
                            tracing::warn!("despawn refused: {e}");
                        }
                    });
                    c.menu.current.clear();
                    c.menu.status = "Vehicle despawned.".into();
                }
            };
            None
        }
        Action::Switch(id) => {
            c.commands.queue(move |w: &mut World| {
                if let Err(e) = ops::switch_to(w, id) {
                    tracing::warn!("switch refused: {e}");
                }
            });
            c.menu.status.clear();
            None
        }
        Action::DespawnId(id) => {
            c.commands
                .queue(move |w: &mut World| crate::session_vehicles::ask_despawn(w, id));
            None
        }
        Action::Respawn(id) => {
            c.commands.queue(move |w: &mut World| {
                if let Err(e) = ops::respawn(w, id) {
                    tracing::warn!("respawn refused: {e}");
                }
            });
            c.menu.status = "Vehicle put back at its spawn point.".into();
            None
        }
        Action::ResetOnRoad(id) => {
            c.commands.queue(move |w: &mut World| {
                if let Err(e) = crate::session_vehicles::reset_on_road(w, id) {
                    tracing::warn!("reset on the road refused: {e}");
                }
            });
            c.menu.status = "Vehicle put on the nearest road.".into();
            None
        }
        other => Some(other),
    }
}

/// Opening and closing the pages of the menu. Returns the action when this group does not handle it.
pub(super) fn navigate_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::OpenSelect => {
            c.menu.page = Page::Select;
            c.menu.status.clear();
            None
        }
        Action::OpenFormation => {
            c.menu.page = Page::Formation;
            c.menu.status.clear();
            None
        }
        Action::Close => {
            c.menu.page = Page::Closed;
            c.menu.config_open = false;
            None
        }
        Action::Back => {
            c.menu.page = if c.menu.page == Page::ChangeMapConfirm {
                Page::ChangeMap
            } else {
                Page::Main
            };
            c.menu.config_open = false;
            None
        }
        Action::OpenRoute => {
            c.menu.page = Page::Route;
            c.menu.status.clear();
            None
        }
        Action::OpenChangeMap => {
            c.menu.page = Page::ChangeMap;
            c.menu.status.clear();
            c.menu.change_map = ChangeMapDraft::default();
            None
        }
        Action::OpenJump => {
            c.menu.page = Page::Jump;
            c.menu.status.clear();
            None
        }
        Action::JumpEntry(id) => {
            c.menu.page = Page::Closed;
            c.commands.queue(move |w: &mut World| jump_to_entry(w, id));
            None
        }
        Action::OpenConfig => {
            c.menu.config_open = true;
            None
        }
        Action::CloseConfig => {
            c.menu.config_open = false;
            None
        }
        Action::ResetConfig => {
            c.menu.setvars.clear();
            None
        }
        other => Some(other),
    }
}

/// The route and timetable pages. Returns the action when this group does not handle it.
pub(super) fn route_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::PickRoute(code) => {
            c.commands.queue(move |w: &mut World| pick_route(w, code));
            None
        }
        Action::PickRouteText(text) => {
            c.menu.page = Page::Route;
            c.commands
                .queue(move |w: &mut World| pick_route_text(w, &text));
            None
        }
        Action::TtLine(i) => {
            c.menu.tt_line = Some(i);
            c.menu.tt_tour = None;
            c.menu.scroll[6] = 0.0;
            None
        }
        Action::TtTour(i) => {
            c.menu.tt_tour = Some(i);
            c.menu.scroll[6] = 0.0;
            None
        }
        Action::TtRun(i) => {
            let (Some(l), Some(t)) = (c.menu.tt_line, c.menu.tt_tour) else {
                return None;
            };
            c.commands.queue(move |w: &mut World| pick_run(w, l, t, i));
            None
        }
        Action::TtUp => {
            if c.menu.tt_tour.take().is_none() {
                c.menu.tt_line = None;
            }
            c.menu.scroll[6] = 0.0;
            None
        }
        Action::ClearRoute => {
            c.commands.queue(|w: &mut World| clear_route(w));
            None
        }
        other => Some(other),
    }
}

/// The change-of-map pages. Returns the action when this group does not handle it.
pub(super) fn change_map_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::PickMap(id) => {
            if c.menu.change_map.map.as_deref() != Some(id.as_str()) {
                c.menu.change_map.entry = c
                    .menu
                    .change_map
                    .entry
                    .take()
                    .filter(|_| c.menu.page == Page::ChangeMap);
            }
            c.menu.change_map.map = Some(id);
            c.menu.page = Page::ChangeMapConfirm;
            c.menu.status.clear();
            None
        }
        Action::PickMapEntry(name) => {
            c.menu.change_map.entry = Some(name);
            None
        }
        Action::DriveWithoutVehicle => {
            c.menu.change_map.without_vehicle = true;
            None
        }
        Action::ConfirmChangeMap => {
            c.commands.queue(|w: &mut World| confirm_change_map(w));
            None
        }
        other => Some(other),
    }
}
