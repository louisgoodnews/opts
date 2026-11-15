//! What the menu's buttons do, and the marker components of its modals.

use super::*;

/// What a button of the menu does.
#[derive(Component, Clone, Debug)]
pub(super) enum Action {
    /// Despawns the active vehicle without asking (`menu.do despawn`).
    Despawn,
    /// Makes a session vehicle the active one.
    Switch(VehicleId),
    /// Asks, then despawns a session vehicle.
    DespawnId(VehicleId),
    /// Puts a session vehicle back at its spawn point.
    Respawn(VehicleId),
    /// Put the vehicle on the nearest road lane under it.
    ResetOnRoad(VehicleId),
    OpenSelect,
    /// Opens the train formation editor.
    OpenFormation,
    Close,
    PickVehicle(usize),
    PickRepaint(String),
    PickEntry(u32),
    /// Spawns at this bus stop (station id) when it is free.
    PickStop(u32),
    OpenConfig,
    CloseConfig,
    ResetConfig,
    Toggle(String),
    /// Moves an option on by `delta` (a choice to its next value, a number by one).
    Step(String, i32),
    Confirm,
    Back,
    /// Opens the *Line and destination* page.
    OpenRoute,
    /// Selects the depot route with this code (`line * 100 + course`).
    PickRoute(i32),
    /// Selects the first depot route whose label contains the text (`menu.do route <text>`).
    PickRouteText(String),
    /// Opens line file `i` of the timetable list.
    TtLine(usize),
    /// Opens tour `i` of the open line file.
    TtTour(usize),
    /// Follows run `i` of the open tour.
    TtRun(usize),
    /// One level up in the timetable list.
    TtUp,
    /// Cancels the route and the timetable.
    ClearRoute,
    /// Opens the *Jump to entry point* page.
    OpenJump,
    /// Jumps to the entry point with this record number.
    JumpEntry(u32),
    /// Opens the *Change map* page.
    OpenChangeMap,
    /// Chooses the target map (folder name) and shows what the change does.
    PickMap(String),
    /// Chooses the start point on the target map.
    PickMapEntry(String),
    /// Goes on without a vehicle on the other map.
    DriveWithoutVehicle,
    /// Changes the map.
    ConfirmChangeMap,
    /// Opens the *Configurations* page.
    OpenConfigs,
    /// Builds the saved configuration with this id as a new session vehicle.
    SpawnConfig(String),
    /// Saves the active vehicle as a configuration.
    SaveConfig,
    /// Deletes the saved configuration with this id (on the second press).
    DeleteConfig(String),
    /// Starts renaming the saved configuration with this id.
    RenameConfig(String),
    /// Applies the typed name.
    CommitRename,
    /// Opens the *Setvars* page.
    OpenSetvars,
    /// Switches an option of the active vehicle on or off, live.
    LiveToggle(String),
    /// Moves an option of the active vehicle on by `delta`, live.
    LiveStep(String, i32),
    /// Puts every option of the active vehicle back to its repaint's value.
    LiveReset,
    /// Leaves the name field without renaming.
    CancelRename,
}

/// Every entity of the menu's modals.
#[derive(Component)]
pub(super) struct MenuRoot;

/// A scrollable list; the index says which of the carried-over offsets it uses.
#[derive(Component)]
pub(super) struct ListScroll(pub(super) usize);
