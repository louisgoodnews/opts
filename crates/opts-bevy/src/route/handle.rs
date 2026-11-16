//! The selection handle: the shared route state and the methods scripts and the game call on it.

use super::*;

mod timetable;

impl RouteHandle {
    /// A handle on a depot with nothing selected.
    #[must_use]
    pub fn new(hof: Option<Arc<HofFile>>) -> Self {
        Self(Arc::new(RwLock::new(RouteState {
            hof,
            ..RouteState::default()
        })))
    }

    /// Reads the state.
    pub fn with<R>(&self, f: impl FnOnce(&RouteState) -> R) -> R {
        match self.0.read() {
            Ok(g) => f(&g),
            Err(p) => f(&p.into_inner()),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut RouteState) -> R) -> R {
        match self.0.write() {
            Ok(mut g) => f(&mut g),
            Err(p) => f(&mut p.into_inner()),
        }
    }

    /// The depot, if the vehicle has one.
    #[must_use]
    pub fn hof(&self) -> Option<Arc<HofFile>> {
        self.with(|s| s.hof.clone())
    }

    /// Selects the route with this code (`line * 100 + course`).
    ///
    /// # Errors
    /// [`RouteError::NoDepot`] without a depot, [`RouteError::UnknownRoute`] when the depot has
    /// no such route (the selection stays as it was).
    pub fn select(&self, route_code: i32) -> Result<RouteSelection, RouteError> {
        self.with_mut(|s| {
            let hof = s.hof.clone().ok_or(RouteError::NoDepot)?;
            let (idx, _) = hof
                .find_route_by_code(route_code)
                .ok_or(RouteError::UnknownRoute(route_code))?;
            s.active_route = Some(idx);
            s.stop_index = 0;
            Ok(())
        })?;
        self.selection().ok_or(RouteError::UnknownRoute(route_code))
    }

    /// Selects the depot's first route (a test or demo stand-in for the driver's choice).
    pub fn with_first_route(&self) {
        let code = self.with(|s| {
            s.hof
                .as_ref()
                .and_then(|h| h.routes.first().map(|r| r.route_code))
        });
        if let Some(c) = code {
            let _ = self.select(c);
        }
    }

    /// Clears the selection (the driver cancelled the route).
    pub fn clear(&self) {
        self.with_mut(|s| {
            s.active_route = None;
            s.stop_index = 0;
        });
    }

    /// The current selection.
    #[must_use]
    pub fn selection(&self) -> Option<RouteSelection> {
        self.with(|s| {
            let r = s.route()?;
            Some(RouteSelection {
                route_code: r.route_code,
                line: r.route_code / 100,
                course: r.route_code % 100,
                terminus_code: r.terminus_code,
                line_string: s.line_text(),
                route_string: r.route_string.clone(),
            })
        })
    }

    /// Every route of the depot as selections, in the depot's order.
    #[must_use]
    pub fn routes(&self) -> Vec<RouteSelection> {
        self.with(|s| {
            s.hof.as_ref().map_or_else(Vec::new, |h| {
                h.routes
                    .iter()
                    .map(|r| RouteSelection {
                        route_code: r.route_code,
                        line: r.route_code / 100,
                        course: r.route_code % 100,
                        terminus_code: r.terminus_code,
                        line_string: if !r.line_string.is_empty()
                            && r.line_string.chars().all(|c| c == 'X')
                        {
                            (r.route_code / 100).to_string()
                        } else {
                            r.line_string.clone()
                        },
                        route_string: r.route_string.clone(),
                    })
                    .collect()
            })
        })
    }
}
