//! Reading the selection: the route, the timetable columns of the active run and the line text.

use super::*;

impl RouteState {
    /// The selected route, if any.
    #[must_use]
    pub fn route(&self) -> Option<&opts_core::formats::hof::HofRoute> {
        self.hof.as_ref()?.route(self.active_route?)
    }

    /// The terminus row of the depot that the active timetable's destination text names (the
    /// trip's title against every string of the terminus rows, ignoring case, spaces and `*`),
    /// or the selected route's terminus without a timetable.
    #[must_use]
    pub fn tt_terminus_index(&self) -> Option<usize> {
        let hof = self.hof.as_ref()?;
        if let Some(sc) = &self.schedule {
            let want = flat(&sc.title);
            if !want.is_empty() {
                if let Some(i) = hof.termini.iter().position(|t| {
                    flat(&t.ident) == want || t.strings.iter().any(|x| flat(x) == want)
                }) {
                    return Some(i);
                }
            }
        }
        let rt = self.route()?;
        hof.find_terminus_by_code(rt.terminus_code).map(|(i, _)| i)
    }

    /// The line text of the active timetable (the trip's), else the route's.
    #[must_use]
    pub fn tt_line_text(&self) -> String {
        match &self.schedule {
            Some(s) if !s.line.is_empty() => s.line.clone(),
            _ => self.line_text(),
        }
    }

    /// Stops of the active timetable, else of the selected route.
    #[must_use]
    pub fn tt_stop_count(&self) -> usize {
        match &self.schedule {
            Some(s) => s.stops.len(),
            None => self.route().map_or(0, |r| r.busstops.len()),
        }
    }

    /// Name of stop `i` of the active timetable (its `Busstops.cfg` name), else of the route.
    #[must_use]
    pub fn tt_stop_name(&self, i: usize) -> String {
        match &self.schedule {
            Some(s) => s.stops.get(i).map(|x| x.name.clone()).unwrap_or_default(),
            None => self
                .route()
                .and_then(|r| r.busstops.get(i).cloned())
                .unwrap_or_default(),
        }
    }

    /// Scheduled arrival at stop `i` (s after midnight); 0 without a timetable.
    #[must_use]
    pub fn tt_arrival_s(&self, i: usize) -> f32 {
        self.schedule
            .as_ref()
            .and_then(|s| s.stops.get(i))
            .map_or(0.0, |x| x.arrival_s)
    }

    /// Scheduled departure from stop `i` (s after midnight); 0 without a timetable.
    #[must_use]
    pub fn tt_departure_s(&self, i: usize) -> f32 {
        self.schedule
            .as_ref()
            .and_then(|s| s.stops.get(i))
            .map_or(0.0, |x| x.departure_s)
    }

    /// The line text shown for the selected route: the depot's, or the line number when the
    /// depot marks "none" with `X`s.
    #[must_use]
    pub fn line_text(&self) -> String {
        self.route().map_or_else(String::new, |r| {
            if !r.line_string.is_empty() && r.line_string.chars().all(|c| c == 'X') {
                (r.route_code / 100).to_string()
            } else {
                r.line_string.clone()
            }
        })
    }
}
