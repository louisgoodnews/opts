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

mod handle;
mod keys;
pub use keys::{has_drucker_keypad, has_ibis_keypad, ibis_keys, KeyQueue};

/// Finds the depot (`.hof`) for a vehicle. `map_names` are the map's names to match, most
/// telling first (its friendly name, then its folder name). For each name in turn: a file of the
/// vehicle's folder named like it, then one whose name contains it (or the other way round);
/// among several the one with the most routes wins, then the longer name: for `Berlin-Spandau`,
/// `Spandau.hof` (the map's lines) and not `Berlin.hof` (destinations only), which came first by
/// name. Next a file of the vehicle's folder sharing tokens with the names (`Hamburg109` and
/// `1_HH109_HHA.hof` share `109`): shared numbers before shared words, then tokens of the
/// vehicle's folder, then the fewest tokens neither names. Then the shared folders by name, and
/// last the first file of the vehicle's folder in name order. Files that do not parse are
/// skipped. The token step is OPTS's guess; OMSI lets the player pick the depot.
#[must_use]
pub fn find_depot(bus_dir: &Path, omsi_root: &Path, map_names: &[&str]) -> Option<HofFile> {
    let maps: Vec<String> = map_names
        .iter()
        .map(|n| n.trim().to_ascii_lowercase())
        .filter(|n| !n.is_empty())
        .collect();
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
    // The best parsed file among `candidates`, by `rank`, then the number of routes, then the
    // longer name.
    let best = |candidates: Vec<([i64; 5], &(String, std::path::PathBuf))>| {
        candidates
            .into_iter()
            .filter_map(|(rank, (stem, p))| {
                parse(p).map(|h| ((rank, h.routes.len(), stem.len()), h))
            })
            .max_by_key(|(key, _)| *key)
            .map(|(_, h)| h)
    };
    let own = hofs_in(bus_dir);
    for map in &maps {
        for (stem, p) in &own {
            if stem == map {
                if let Some(h) = parse(p) {
                    return Some(h);
                }
            }
        }
        let named = own
            .iter()
            .filter(|(stem, _)| stem.contains(map.as_str()) || map.contains(stem.as_str()))
            .map(|f| ([0; 5], f))
            .collect();
        if let Some(h) = best(named) {
            return Some(h);
        }
    }
    let wanted: std::collections::HashSet<String> =
        maps.iter().flat_map(|m| name_tokens(m)).collect();
    let folder = bus_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let vehicle: std::collections::HashSet<String> = name_tokens(&folder).collect();
    let shared = own
        .iter()
        .filter_map(|f| {
            // Shared numbers (a line or edition) first, then shared words, then tokens of the
            // vehicle's folder (the operator: `HHA` in `HH109_Stadtbus_HHA`); then the fewest
            // numbers and words neither names (`1_HH109-2_HHA` and `1_HH109-VAN_HHA` lose to
            // `1_HH109_HHA` for `Hamburg109`).
            let mut rank = [0_i64; 5];
            for t in name_tokens(&f.0) {
                let number = t.chars().all(char::is_numeric);
                match (wanted.contains(&t), vehicle.contains(&t), number) {
                    (true, _, true) => rank[0] += 1,
                    (true, _, false) => rank[1] += 1,
                    (false, true, _) => rank[2] += 1,
                    (false, false, true) => rank[3] -= 1,
                    (false, false, false) => rank[4] -= 1,
                }
            }
            (rank[0] + rank[1] > 0).then_some((rank, f))
        })
        .collect();
    if let Some(h) = best(shared) {
        return Some(h);
    }
    if let Some(h) = own.iter().find_map(|(_, p)| parse(p)) {
        return Some(h);
    }
    for map in &maps {
        for dir in [
            omsi_root.join("Vehicles"),
            omsi_root.join("Vehicles/.hof Dateien"),
        ] {
            for (stem, p) in hofs_in(&dir) {
                if stem == *map || stem.contains(map.as_str()) {
                    if let Some(h) = parse(&p) {
                        return Some(h);
                    }
                }
            }
        }
    }
    None
}

/// The word and number tokens of a lower-case name: runs of letters (two or more) and runs of
/// digits, so `1_hh109-2_hha` gives `1`, `hh`, `109`, `2` and `hha`.
fn name_tokens(name: &str) -> impl Iterator<Item = String> + '_ {
    name.split(|c: char| !c.is_alphanumeric())
        .flat_map(|word| {
            let mut runs = Vec::new();
            let mut cur = String::new();
            for c in word.chars() {
                if !cur.is_empty() && cur.chars().all(char::is_numeric) != c.is_numeric() {
                    runs.push(std::mem::take(&mut cur));
                }
                cur.push(c);
            }
            runs.push(cur);
            runs
        })
        .filter(|t| t.chars().all(char::is_numeric) && !t.is_empty() || t.chars().count() >= 2)
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

/// The route of a vehicle in the world: the shared state and the keys still to be pressed.
#[derive(bevy::prelude::Component, Debug, Clone, Default)]
pub struct VehicleRoute {
    /// Depot and selection.
    pub handle: RouteHandle,
    /// Keys waiting to be pressed on the vehicle's IBIS.
    pub keys: KeyQueue,
    /// Seconds the target is still offered to the displays ([`keep_target`]).
    pub pending_target_s: f32,
    /// The keys of a selection are being typed; the target goes to the displays after them.
    pub offer_after_typing: bool,
}

impl VehicleRoute {
    /// A vehicle's route on this handle, nothing to type.
    #[must_use]
    pub fn new(handle: RouteHandle) -> Self {
        Self {
            handle,
            keys: KeyQueue::default(),
            pending_target_s: 0.0,
            offer_after_typing: false,
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
    if route.keys.is_typing() {
        // A driver keys the route first; the displays are told the target when the keys are
        // done (a scheduled target set now would shut the IBIS keypad of vehicles such as the
        // NF6D, which accepts keys only while no target is set).
        route.offer_after_typing = true;
    } else if result.is_ok() || matches!(result, Err(RouteError::NoKeypad)) {
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
        if !route.keys.is_typing() && route.pending_target_s <= 0.0 && !route.offer_after_typing {
            continue;
        }
        if let Ok(mut engine) = scripts.engine.lock() {
            route.keys.press_next(&mut engine, time.delta_secs());
            if route.keys.is_typing() {
                continue;
            }
            if route.offer_after_typing {
                route.offer_after_typing = false;
                schedule_target(&mut engine, &route.handle);
                route.pending_target_s = TARGET_OFFER_S;
            }
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
