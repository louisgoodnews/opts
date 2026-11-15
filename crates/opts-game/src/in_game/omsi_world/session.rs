//! What a world session loads: the resolved content paths and the menu's request.

use super::*;

/// Where the session content comes from, resolved from CLI arguments.
#[derive(Clone)]
pub struct OmsiSessionConfig {
    /// OMSI installation root.
    pub root: PathBuf,
    /// Map folder (`<root>/maps/<name>`).
    pub map_dir: PathBuf,
    /// Explicit `.bus` file, if given.
    pub vehicle_path: Option<PathBuf>,
    /// Entry point name chosen in the New Game screen.
    pub entry_point: String,
    /// The station id of a chosen trip's first stop (M-0145): the drive starts at that bus stop
    /// object instead of the entry point.
    pub start_station: Option<u32>,
    /// Repaint of the player's vehicle (empty: the default of its `.bus`).
    pub livery: String,
    /// Equipment options (`[setvar]`-style script variables) set on the vehicle's scripts after
    /// the repaint's own.
    pub setvars: Vec<(String, i32)>,
    /// `.hum` profile of the player's character, if the installation has one.
    pub human_path: Option<PathBuf>,
    /// Cached path index (the SQLite catalogue), if available.
    pub lookup: Option<opts_bevy::install::PathLookup>,
    /// Cached by-name index for repairing wrong references.
    pub name_lookup: Option<opts_bevy::install::NameLookup>,
    /// Cached all-files-by-name index (plain twins of scrambled meshes).
    pub twin_lookup: Option<opts_bevy::install::TwinLookup>,
    /// Finder of files installed under other folder names than their references say.
    pub relocate_lookup: Option<opts_bevy::install::RelocateLookup>,
}

/// Inputs for [`resolve_session_config`]: CLI overrides plus the menu selection.
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionRequest<'a> {
    /// `--map-path` override.
    pub map_path: Option<&'a Path>,
    /// `--omsi-path` override.
    pub omsi_path: Option<&'a Path>,
    /// `--vehicle-path` override.
    pub vehicle_path: Option<&'a Path>,
    /// Map chosen in the New Game screen (friendly name, name or folder id).
    pub selected_map: &'a str,
    /// Vehicle chosen in the New Game screen (full name or catalogue id).
    pub selected_vehicle: &'a str,
    /// Entry point chosen in the New Game screen (name from `global.cfg`).
    pub selected_entry_point: &'a str,
    /// Repaint chosen in the New Game screen (empty: the vehicle's default).
    pub selected_livery: &'a str,
    /// The session starts with the map only: no vehicle file is resolved and none is warned about.
    pub no_vehicle: bool,
}
