//! Keeps the offered runs current for the chosen map, bus, clock and weekday.

use super::*;

/// What the list was made for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TripKey {
    map: String,
    clock_s: u32,
    weekday: u32,
}

/// The runs offered for the current choices.
#[derive(Resource, Default)]
pub struct TripState {
    key: Option<TripKey>,
    /// The runs, earliest first; empty when none fit (the step is then not offered).
    pub trips: Vec<TripOption>,
    /// The map's timetables are being read.
    pub loading: bool,
    timetables: HashMap<String, Option<Arc<Timetables>>>,
    task: Option<(String, Task<Option<Timetables>>)>,
}

impl TripState {
    /// The offered run with this list id.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&TripOption> {
        self.trips.iter().find(|t| t.choice.id() == id)
    }

    /// Timetables already read for a map (tests).
    pub fn insert_timetables(&mut self, map_id: &str, tt: Timetables) {
        self.timetables
            .insert(map_id.to_string(), Some(Arc::new(tt)));
    }
}

/// Whether the plan drives a bus: a road vehicle alone (no train, no drive without a vehicle).
pub(super) fn drives_a_bus(setup: &NewGameSetup, catalogue: &GameCatalogue) -> bool {
    !setup.no_vehicle
        && setup.formation.is_empty()
        && catalogue
            .find_vehicle(&setup.selected_vehicle)
            .is_some_and(|v| v.class != "rail" && v.class != "aircraft")
}

/// ISO weekday of the plan's date (1 = Monday).
pub(super) fn weekday_of(setup: &NewGameSetup) -> u32 {
    use chrono::Datelike;
    chrono::NaiveDate::from_ymd_opt(setup.year, setup.month, setup.day)
        .map_or(1, |d| d.weekday().number_from_monday())
}

/// Keeps the offered runs current: reads a map's timetables in the background, lists the runs
/// for map, bus, clock and weekday, drops a chosen trip that no longer fits, and leaves the step
/// when it is no longer offered.
pub fn refresh_trips(
    mut trips: ResMut<TripState>,
    mut setup: ResMut<NewGameSetup>,
    catalogue: Res<GameCatalogue>,
    mut state: ResMut<PlannerState>,
) {
    let mut arrived = false;
    if let Some((map, task)) = trips.task.as_mut() {
        if let Some(tt) = check_ready(task) {
            let map = map.clone();
            trips.timetables.insert(map, tt.map(Arc::new));
            trips.task = None;
            arrived = true;
        }
    }
    let map = catalogue
        .map_by_name(&setup.selected_map)
        .map(|m| m.id.clone());
    let key = map
        .filter(|_| drives_a_bus(&setup, &catalogue))
        .map(|map| TripKey {
            map,
            clock_s: setup.hour * 3_600 + setup.minute * 60,
            weekday: weekday_of(&setup),
        });
    if key == trips.key && !arrived {
        return;
    }
    trips.key.clone_from(&key);
    let list = match &key {
        None => Some(Vec::new()),
        Some(k) => match trips.timetables.get(&k.map) {
            Some(tt) => Some(
                tt.as_deref()
                    .map(|tt| list_trips(tt, k.clock_s, k.weekday))
                    .unwrap_or_default(),
            ),
            None => {
                if trips.task.as_ref().is_none_or(|(m, _)| *m != k.map) {
                    let map = k.map.clone();
                    let task = IoTaskPool::get().spawn(async move {
                        crate::omsi_root::known_omsi_root()
                            .map(|root| root.join("maps").join(&map))
                            .and_then(|d| opts_bevy::route::find_timetables(&d))
                    });
                    trips.task = Some((k.map.clone(), task));
                }
                None
            }
        },
    };
    let loading = list.is_none();
    if trips.loading != loading {
        trips.loading = loading;
    }
    let Some(list) = list else {
        // Asked again when the timetables arrive.
        trips.key = None;
        return;
    };
    let offered = !list.is_empty();
    if setup
        .trip
        .as_ref()
        .is_some_and(|c| !list.iter().any(|t| t.choice == *c))
    {
        setup.trip = None;
    }
    trips.trips = list;
    if state.trips_offered != offered {
        state.trips_offered = offered;
        state.built = None;
    }
    if !offered && state.step == PlannerStep::Trip {
        state.step = PlannerStep::Review;
    }
}
