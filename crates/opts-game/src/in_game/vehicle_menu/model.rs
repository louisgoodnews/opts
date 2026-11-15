//! The menu's state: the pages, the choices and the carried-over scroll positions.

use super::*;

/// Page of the menu that is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    /// Not shown.
    #[default]
    Closed,
    /// *Despawn* / *Spawn* / *Close*.
    Main,
    /// The three-column *Select Vehicle* modal.
    Select,
    /// The train formation editor (`formation_menu`), reached from the Select Vehicle modal.
    Formation,
    /// *Line and destination*: the depot's routes and the map's timetables (M-0099).
    Route,
    /// *Jump to entry point*: every entry point of the map; the camera goes there and the active
    /// vehicle, if there is one, is put there.
    Jump,
    /// *Change map*: the installed maps (M-0108).
    ChangeMap,
    /// *Change to <map>*: what the change does and the one question.
    ChangeMapConfirm,
    /// *Configurations*: the saved bus configurations (M-0077).
    Configs,
    /// *Setvars*: the equipment options of the active vehicle, changed while it runs (M-0110).
    Setvars,
}

/// State of the menu.
#[derive(Resource, Default)]
pub struct VehicleMenu {
    pub(super) page: Page,
    pub(super) config_open: bool,
    /// Index into [`GameCatalogue::vehicles`].
    pub(super) vehicle: Option<usize>,
    /// Chosen repaint (empty: the vehicle's default).
    pub(super) repaint: String,
    pub(super) entry: Option<u32>,
    /// The bus stop chosen instead of an entry point (station id, M-0145).
    pub(super) stop: Option<u32>,
    /// The map's bus stops (station id and label, from the timetables), read once.
    pub(super) shown_stops: Vec<(u32, String)>,
    /// Equipment options the player changed.
    pub(super) setvars: HashMap<String, i32>,
    /// Entry points shown in the last build (rebuild when the free ones change).
    pub(super) shown_entries: Vec<(u32, String)>,
    /// Repaints of the chosen vehicle (read from disk once per choice).
    pub(super) repaints: Option<(usize, VehicleRepaints)>,
    pub(super) status: String,
    /// Scroll offsets carried over a rebuild: vehicles, repaints, entries, description,
    /// configuration.
    pub(super) scroll: [f32; 7],
    /// Name of the vehicle spawned through the menu.
    pub(super) current: String,
    pub(super) built: Option<(Page, bool)>,
    /// Entries resolved and whether the index has finished, in the last build.
    pub(super) index_progress: (usize, bool),
    pub(super) entry_total: usize,
    /// Text typed into the vehicle search field.
    pub(super) search: String,
    /// The session vehicle rows shown in the last build of the main page (rebuild on change).
    pub(super) shown_rows: Vec<RowSig>,
    /// Text typed into the route search field.
    pub(super) route_search: String,
    /// The timetable line file and tour the right column has opened (indices into the map's
    /// line files and that file's tours).
    pub(super) tt_line: Option<usize>,
    pub(super) tt_tour: Option<usize>,
    /// The choices of the *Change map* pages.
    pub(super) change_map: ChangeMapDraft,
    /// The rows of the *Configurations* page.
    pub(super) configs: ConfigsView,
    /// The repaints the *Setvars* page offers its options from (read once per vehicle).
    pub(super) live_setvars: LiveSetvars,
}

/// What a row of the main page shows; a change rebuilds the page.
pub(super) type RowSig = (VehicleId, bool, String);

impl VehicleMenu {
    /// Opens the main page (*Despawn* / *Spawn* / *Close*), as Left Alt does.
    pub fn open_main(&mut self) {
        self.page = Page::Main;
        self.status.clear();
    }

    /// Whether any page is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.page != Page::Closed
    }

    /// Whether the formation editor is the open page.
    #[must_use]
    pub fn formation_open(&self) -> bool {
        self.page == Page::Formation
    }

    /// Goes back from the formation editor to the Select Vehicle modal.
    pub fn leave_formation(&mut self) {
        if self.page == Page::Formation {
            self.page = Page::Select;
        }
    }

    /// Closes the menu.
    pub fn close(&mut self) {
        self.page = Page::Closed;
        self.config_open = false;
    }

    /// The free entry points of the map (record number and label).
    #[must_use]
    pub fn free_entries(&self) -> &[(u32, String)] {
        &self.shown_entries
    }

    /// The entry point chosen.
    #[must_use]
    pub fn entry(&self) -> Option<u32> {
        self.entry
    }

    /// Chooses an entry point.
    pub fn set_entry(&mut self, id: u32) {
        self.entry = Some(id);
    }

    /// Entries resolved so far and whether the index has finished.
    #[must_use]
    pub fn entry_progress(&self) -> (usize, bool) {
        self.index_progress
    }

    /// The state as JSON (`menu.state`).
    #[must_use]
    pub fn describe(&self) -> serde_json::Value {
        serde_json::json!({
            "page": format!("{:?}", self.page),
            "config_open": self.config_open,
            "vehicle": self.vehicle,
            "repaint": self.repaint,
            "entry": self.entry,
            "stop": self.stop,
            "stops": self.shown_stops,
            "free_entries": self.shown_entries,
            "setvars": self.setvars,
            "status": self.status,
        })
    }
}
