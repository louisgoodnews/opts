//! The route of a vehicle (M-0099): its depot (`.hof`), the line and course the driver has
//! selected, the stop it is at, and the keypad sequence that makes the vehicle's own IBIS accept
//! a selection.
//!
//! The data is a service of its own (strategy decision D7): [`RouteHandle`] is shared between
//! the callbacks the scripts call (`GetRouteIndex`, `GetTT*`, ...), the control protocol and the
//! game, so a change of the selection is seen by the very next script call. The scripts own the
//! displays; the host only hands them data and the driver's inputs.

use std::path::Path;
use std::sync::{Arc, RwLock};

use opts_core::formats::hof::HofFile;
use opts_core::formats::timetable::{ActiveSchedule, Timetables};
use opts_sim::script::ScriptEngine;

/// What the driver has selected, as the scripts and the protocol name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSelection {
    /// Route code as keyed on the IBIS: `line * 100 + course` (`13001`: line 130, course 01).
    pub route_code: i32,
    /// Line (`route_code / 100`).
    pub line: i32,
    /// Course (`route_code % 100`).
    pub course: i32,
    /// Destination code of the route (the terminus table's code column).
    pub terminus_code: i32,
    /// Line text of the route (`130`, `91.06A`; a depot may mark "no text" with `X`s).
    pub line_string: String,
    /// Route name of the depot (`URUH-NERV`).
    pub route_string: String,
}

/// Why a selection or a lookup failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    /// The vehicle has no depot file.
    NoDepot,
    /// The depot has no route with this code.
    UnknownRoute(i32),
    /// The vehicle's scripts have no IBIS keypad this layer knows (`ibis_eingabe`, `ibis_0`...).
    NoKeypad,
    /// The map has no timetable files.
    NoTimetables,
    /// The map's timetables have no such run (`line file/tour/run`).
    UnknownRun(String),
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDepot => write!(f, "the vehicle has no depot (.hof) file"),
            Self::UnknownRoute(c) => write!(f, "the depot has no route {c}"),
            Self::NoKeypad => write!(f, "the vehicle's scripts have no IBIS keypad"),
            Self::NoTimetables => write!(f, "the map has no timetable files (TTData)"),
            Self::UnknownRun(r) => write!(f, "the timetables have no run {r}"),
        }
    }
}

impl std::error::Error for RouteError {}

/// The state behind a [`RouteHandle`].
#[derive(Debug, Default)]
pub struct RouteState {
    /// The depot the vehicle belongs to.
    pub hof: Option<Arc<HofFile>>,
    /// Index into the depot's routes of the selected route.
    pub active_route: Option<usize>,
    /// The stop the vehicle is at (index along the route).
    pub stop_index: usize,
    /// Delay against the timetable (s, late positive); free driving has none.
    pub delay_s: f32,
    /// The map's timetable files, if it has any.
    pub timetables: Option<Arc<Timetables>>,
    /// The run the driver follows, if a timetable is selected.
    pub schedule: Option<ActiveSchedule>,
}

/// Lower case without spaces and `*` (platform and padding marks): destination texts of the
/// timetable files and of the depot differ in both.
fn flat(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && *c != '*')
        .collect::<String>()
        .to_lowercase()
}

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

/// A shared route state; clones refer to the same state.
#[derive(Debug, Clone, Default)]
pub struct RouteHandle(Arc<RwLock<RouteState>>);

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

    /// The map's timetable files.
    pub fn set_timetables(&self, t: Option<Arc<Timetables>>) {
        self.with_mut(|s| s.timetables = t);
    }

    /// The map's timetable files.
    #[must_use]
    pub fn timetables(&self) -> Option<Arc<Timetables>> {
        self.with(|s| s.timetables.clone())
    }

    /// Follows run `run` of tour `tour` in line file `line_file`: the schedule becomes active,
    /// the stop is the first, and the depot's route of that line and destination is selected when
    /// it has one (so the IBIS can be keyed). Returns the schedule and the route code selected.
    ///
    /// # Errors
    /// [`RouteError::UnknownRun`] when the map has no such run or no timetable files.
    pub fn select_run(
        &self,
        line_file: &str,
        tour: usize,
        run: usize,
    ) -> Result<(ActiveSchedule, Option<i32>), RouteError> {
        let tt = self.timetables().ok_or(RouteError::NoTimetables)?;
        let schedule = tt
            .schedule(line_file, tour, run)
            .ok_or_else(|| RouteError::UnknownRun(format!("{line_file}/{tour}/{run}")))?;
        self.with_mut(|s| {
            s.schedule = Some(schedule.clone());
            s.stop_index = 0;
        });
        let code = self.route_code_for(&schedule);
        if let Some(c) = code {
            let _ = self.select(c);
            // `select` resets the stop; the schedule stays.
        }
        Ok((schedule, code))
    }

    /// Stops following a timetable.
    pub fn clear_schedule(&self) {
        self.with_mut(|s| {
            s.schedule = None;
            s.delay_s = 0.0;
        });
    }

    /// The run followed, if any.
    #[must_use]
    pub fn schedule(&self) -> Option<ActiveSchedule> {
        self.with(|s| s.schedule.clone())
    }

    /// The depot route (route code) that has the schedule's line and destination: the terminus
    /// the trip's title names, on a route whose line text is the trip's line (or whose line
    /// number is it).
    #[must_use]
    pub fn route_code_for(&self, schedule: &ActiveSchedule) -> Option<i32> {
        self.with(|s| {
            let hof = s.hof.as_ref()?;
            let want = flat(&schedule.title);
            let terminus = hof
                .termini
                .iter()
                .find(|t| {
                    !want.is_empty()
                        && (flat(&t.ident) == want || t.strings.iter().any(|x| flat(x) == want))
                })
                .map(|t| t.code)?;
            let line = flat(&schedule.line);
            hof.routes
                .iter()
                .find(|r| {
                    r.terminus_code == terminus
                        && (flat(&r.line_string) == line
                            || (r.route_code / 100).to_string() == line)
                })
                .map(|r| r.route_code)
        })
    }

    /// Sets the clock the delay is measured against (s after midnight) and recomputes the delay
    /// of a followed timetable: now minus the scheduled arrival at the current stop (0 in free
    /// driving).
    pub fn set_clock(&self, seconds_of_day: f32) {
        self.with_mut(|s| {
            let stop = s.stop_index;
            s.delay_s = match &s.schedule {
                Some(sc) => sc
                    .stops
                    .get(stop)
                    .map_or(0.0, |x| seconds_of_day - x.arrival_s),
                None => 0.0,
            };
        });
    }

    /// Moves to stop `index` of the route.
    pub fn set_stop(&self, index: usize) {
        self.with_mut(|s| s.stop_index = index);
    }

    /// Sets the delay against the timetable (s).
    pub fn set_delay(&self, delay_s: f32) {
        self.with_mut(|s| s.delay_s = delay_s);
    }
}

/// Finds the depot (`.hof`) for a vehicle: a file of the vehicle's folder named like the map,
/// one whose name contains the map's (or the other way round), the shared folders by the same
/// rule, and last the first file of the vehicle's folder in name order. Files that do not parse
/// are skipped.
#[must_use]
pub fn find_depot(bus_dir: &Path, omsi_root: &Path, map_name: &str) -> Option<HofFile> {
    let map = map_name.trim().to_ascii_lowercase();
    let parse = |p: &Path| {
        std::fs::read(p)
            .ok()
            .and_then(|b| HofFile::parse_bytes(&b).ok())
    };
    let hofs_in = |dir: &Path| -> Vec<(String, std::path::PathBuf)> {
        let mut v: Vec<_> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("hof")))
            .map(|p| {
                let stem = p
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                (stem, p)
            })
            .collect();
        v.sort();
        v
    };
    let own = hofs_in(bus_dir);
    if !map.is_empty() {
        for (stem, p) in &own {
            if *stem == map {
                if let Some(h) = parse(p) {
                    return Some(h);
                }
            }
        }
        for (stem, p) in &own {
            if stem.contains(&map) || map.contains(stem.as_str()) {
                if let Some(h) = parse(p) {
                    return Some(h);
                }
            }
        }
    }
    if let Some(h) = own.iter().find_map(|(_, p)| parse(p)) {
        return Some(h);
    }
    if !map.is_empty() {
        for dir in [
            omsi_root.join("Vehicles"),
            omsi_root.join("Vehicles/.hof Dateien"),
        ] {
            for (stem, p) in hofs_in(&dir) {
                if stem == map || stem.contains(&map) {
                    if let Some(h) = parse(&p) {
                        return Some(h);
                    }
                }
            }
        }
    }
    None
}

/// What the depot gives the scripts' predefined strings (`Yard`, `SetLineTo`, `Ident`) and the
/// `schedule_active` flag, for the current selection.
pub fn apply_to_scripts(engine: &mut ScriptEngine, route: &RouteHandle) {
    route.with(|s| {
        if let Some(h) = &s.hof {
            engine.set_string("Yard", &h.name);
        }
        match s.route() {
            Some(r) => {
                engine.set_string("SetLineTo", &s.tt_line_text());
                engine.set_string("Ident", &r.route_code.to_string());
                engine.set("schedule_active", 1.0);
            }
            None if s.schedule.is_some() => {
                // A timetable without a depot route (the depot lacks the line or destination).
                engine.set_string("SetLineTo", &s.tt_line_text());
                engine.set("schedule_active", 1.0);
            }
            None => engine.set("schedule_active", 0.0),
        }
    });
}

/// What the host feeds every frame from the route: `AI_Target_Index` (the terminus row of the
/// selected route, -1 without one) and `schedule_active`.
pub fn feed_route(inputs: &mut opts_sim::script::inputs::HostInputs, route: &RouteHandle) {
    let target = route.with(RouteState::tt_terminus_index);
    inputs.ai_target_index = target.map_or(-1.0, |i| i as f32);
    inputs.schedule_active = route.with(|s| s.active_route.is_some() || s.schedule.is_some());
}

/// Hands the selected route to the vehicle's displays the way OMSI hands a scheduled target to
/// them: `AI_Target_Index` (the terminus row), `SetLineTo` (the line text) and `Ident` set, the
/// trigger `ai_scheduled_settarget` raised. The destination blind and matrix of a vehicle (and
/// the `target_index_int` they write) follow from that. Returns whether a script answered the
/// trigger.
pub fn schedule_target(engine: &mut ScriptEngine, route: &RouteHandle) -> bool {
    apply_to_scripts(engine, route);
    let target = route.with(RouteState::tt_terminus_index);
    match target {
        Some(i) => {
            engine.set("ai_target_index", i as f32);
            engine.trigger("ai_scheduled_settarget")
        }
        None => false,
    }
}

/// Seconds a selected target is offered to the displays again and again: a destination display
/// that is still switching on or winding ignores `ai_scheduled_settarget` (the MAN matrix and the
/// Hamburg flip-dot matrix check that they are idle), so the host asks until the display shows
/// the target (`target_index_int` equals the terminus row) or the time is up.
pub const TARGET_OFFER_S: f32 = 30.0;

/// One step of offering the target again; `pending` counts the seconds left (0: nothing to do).
pub fn keep_target(engine: &mut ScriptEngine, route: &RouteHandle, pending: &mut f32, dt: f32) {
    if *pending <= 0.0 {
        return;
    }
    *pending -= dt;
    let want = route.with(RouteState::tt_terminus_index);
    let Some(want) = want else {
        *pending = 0.0;
        return;
    };
    if (engine.get("target_index_int") - want as f32).abs() < 0.5 {
        *pending = 0.0;
    } else {
        engine.trigger("ai_scheduled_settarget");
    }
}

/// The keys a driver presses on the MAN-style IBIS (`ibis_setmode_linie_kurs`, `ibis_eingabe`,
/// `ibis_<digit>`) to select a route: line and suffix as `line * 100 + suffix`, then the route
/// (course), then the destination (terminus code). Names are the triggers of the scripts.
#[must_use]
pub fn ibis_keys(selection: &RouteSelection) -> Vec<String> {
    let digits = |n: i32| -> Vec<String> {
        n.max(0)
            .to_string()
            .chars()
            .map(|c| format!("ibis_{c}"))
            .collect()
    };
    let mut keys = vec!["ibis_setmode_linie_kurs".to_string()];
    keys.extend(digits(selection.line * 100));
    keys.push("ibis_eingabe".into());
    keys.push("ibis_setmode_route".into());
    keys.extend(digits(selection.course));
    keys.push("ibis_eingabe".into());
    keys.push("ibis_setmode_ziel".into());
    keys.extend(digits(selection.terminus_code));
    keys.push("ibis_eingabe".into());
    keys
}

/// Whether the scripts have the keypad [`ibis_keys`] presses.
#[must_use]
pub fn has_ibis_keypad(engine: &ScriptEngine) -> bool {
    ["ibis_eingabe", "ibis_setmode_linie_kurs", "ibis_0"]
        .iter()
        .all(|t| engine.has_trigger(t))
}

/// Keys still to be pressed, one per frame (the scripts see every key in a frame of its own, as
/// with a person's finger).
#[derive(Debug, Clone, Default)]
pub struct KeyQueue {
    keys: std::collections::VecDeque<String>,
}

impl KeyQueue {
    /// Queues the keys of a selection for this vehicle.
    ///
    /// # Errors
    /// [`RouteError::NoKeypad`] when the scripts have no IBIS keypad of the known kind.
    pub fn type_selection(
        &mut self,
        engine: &ScriptEngine,
        selection: &RouteSelection,
    ) -> Result<(), RouteError> {
        if !has_ibis_keypad(engine) {
            return Err(RouteError::NoKeypad);
        }
        self.keys.clear();
        self.keys.extend(ibis_keys(selection));
        Ok(())
    }

    /// Presses the next key, if any; returns whether one was pressed.
    pub fn press_next(&mut self, engine: &mut ScriptEngine) -> bool {
        match self.keys.pop_front() {
            Some(k) => {
                engine.trigger(&k);
                true
            }
            None => false,
        }
    }

    /// Whether keys are still waiting.
    #[must_use]
    pub fn is_typing(&self) -> bool {
        !self.keys.is_empty()
    }
}

/// The route of a vehicle in the world: the shared state and the keys still to be pressed.
#[derive(bevy::prelude::Component, Debug, Clone, Default)]
pub struct VehicleRoute {
    /// Depot and selection.
    pub handle: RouteHandle,
    /// Keys waiting to be pressed on the vehicle's IBIS.
    pub keys: KeyQueue,
    /// Seconds the target is still offered to the displays ([`keep_target`]).
    pub pending_target_s: f32,
}

impl VehicleRoute {
    /// A vehicle's route on this handle, nothing to type.
    #[must_use]
    pub fn new(handle: RouteHandle) -> Self {
        Self {
            handle,
            keys: KeyQueue::default(),
            pending_target_s: 0.0,
        }
    }
}

/// Selects route `route_code` for a vehicle: the depot's selection changes at once (the
/// callbacks see it) and the keys of its IBIS are queued, so the scripts take the route the way
/// a driver would enter it.
///
/// # Errors
/// See [`RouteHandle::select`]; [`RouteError::NoKeypad`] leaves the selection made (a vehicle
/// without a known IBIS still answers the timetable callbacks) but queues nothing.
pub fn select_route(
    route: &mut VehicleRoute,
    engine: &ScriptEngine,
    route_code: i32,
) -> Result<RouteSelection, RouteError> {
    let selection = route.handle.select(route_code)?;
    route.keys.type_selection(engine, &selection)?;
    Ok(selection)
}

/// [`select_route`] on a vehicle whose engine is at hand: also hands the target to the displays
/// ([`schedule_target`]).
///
/// # Errors
/// As [`select_route`].
pub fn select_route_on(
    route: &mut VehicleRoute,
    engine: &mut ScriptEngine,
    route_code: i32,
) -> Result<RouteSelection, RouteError> {
    let result = select_route(route, engine, route_code);
    if result.is_ok() || matches!(result, Err(RouteError::NoKeypad)) {
        schedule_target(engine, &route.handle);
        route.pending_target_s = TARGET_OFFER_S;
    }
    result
}

/// Presses one queued key per frame on every vehicle that has some.
pub fn route_typing_system(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    mut query: bevy::prelude::Query<(&mut VehicleRoute, &crate::VehicleScripts)>,
) {
    for (mut route, scripts) in &mut query {
        if !route.keys.is_typing() && route.pending_target_s <= 0.0 {
            continue;
        }
        if let Ok(mut engine) = scripts.engine.lock() {
            route.keys.press_next(&mut engine);
            let handle = route.handle.clone();
            keep_target(
                &mut engine,
                &handle,
                &mut route.pending_target_s,
                time.delta_secs(),
            );
        }
    }
}

/// The timetable files of a map: its `TTData` folder (any letter case), `None` when it has none.
#[must_use]
pub fn find_timetables(map_dir: &Path) -> Option<Timetables> {
    let dir = std::fs::read_dir(map_dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("TTData"))
        })?;
    let t = Timetables::load_dir(&dir);
    (!t.is_empty()).then_some(t)
}

/// [`select_route`] for a run of the map's timetable: the run becomes the active timetable
/// (`GetTT*`), the depot's matching route is selected and keyed on the IBIS when the vehicle has
/// one, and the target is offered to the displays.
///
/// # Errors
/// [`RouteError::NoTimetables`] or [`RouteError::UnknownRun`].
pub fn select_run_on(
    route: &mut VehicleRoute,
    engine: &mut ScriptEngine,
    line_file: &str,
    tour: usize,
    run: usize,
) -> Result<(ActiveSchedule, Option<i32>), RouteError> {
    let (schedule, code) = route.handle.select_run(line_file, tour, run)?;
    if code.is_some() {
        if let Some(sel) = route.handle.selection() {
            // A vehicle without the known keypad still follows the timetable.
            let _ = route.keys.type_selection(engine, &sel);
        }
    }
    schedule_target(engine, &route.handle);
    route.pending_target_s = TARGET_OFFER_S;
    Ok((schedule, code))
}

/// Keeps the delay of every followed timetable current from the simulation clock.
pub fn route_clock_system(
    clock: Option<bevy::prelude::Res<crate::scenery_runtime::SceneryClock>>,
    query: bevy::prelude::Query<&VehicleRoute>,
) {
    let Some(clock) = clock else { return };
    let now = (clock.seconds_of_day % 86_400.0) as f32;
    for r in &query {
        r.handle.set_clock(now);
    }
}
