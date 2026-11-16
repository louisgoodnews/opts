//! The route and timetable actions of the menu.

use super::super::super::route_page::{clear_route, pick_route, pick_route_text, pick_run};
use super::super::*;

/// The route and timetable pages. Returns the action when this group does not handle it.
pub(in super::super) fn route_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
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
