//! Command-line arguments of the game binary (`opts`), including the control-server and hot-reload switches.

use clap::Parser;
use std::path::PathBuf;

/// The smallest window the interface is designed for, in logical pixels (width, height).
pub const MIN_WINDOW_SIZE: (f32, f32) = (1024.0, 640.0);

/// Command-line arguments for the Open Public Transport Simulator (`opts`).
#[derive(Parser, Debug, Clone, PartialEq)]
#[command(
    name = "opts",
    author = "OPTS Contributors",
    version,
    about = "Open Public Transport Simulator — Bringing OMSI 2 assets to Bevy",
    long_about = "Interactive 3D simulation and main menu for Open Public Transport Simulator."
)]
pub struct CliArgs {
    /// Override the map path for the background preview and initial session.
    #[arg(long, value_name = "PATH")]
    pub map_path: Option<PathBuf>,

    /// OMSI installation root. Derived from `--map-path` (`<root>/maps/<Map>`) when omitted.
    #[arg(long, value_name = "PATH")]
    pub omsi_path: Option<PathBuf>,

    /// Player vehicle `.bus` file. Falls back to the first vehicle in the map's `ailists.cfg`.
    #[arg(long, value_name = "FILE")]
    pub vehicle_path: Option<PathBuf>,

    /// Skip the menus and start the in-game session immediately.
    #[arg(long, default_value_t = false)]
    pub autostart: bool,

    /// Start the drive when the load ends with problems that would wait for *Continue* (broken
    /// tiles, a missing vehicle model part ...): they are logged and the drive begins. A load that
    /// failed outright still stops.
    #[arg(long, default_value_t = false)]
    pub accept_load_problems: bool,

    /// Save a screenshot of the running game to this PNG path once loading finishes, then exit.
    #[arg(long, value_name = "FILE")]
    pub screenshot: Option<PathBuf>,

    /// Seconds to wait after the HUD appears before taking `--screenshot` (default 4).
    #[arg(long, value_name = "SECS", default_value_t = 4.0)]
    pub screenshot_delay: f32,

    /// Initial camera mode for the in-game session: `external` (default), `ego` or `free`.
    #[arg(long, value_name = "MODE")]
    pub camera: Option<String>,

    /// Persist the specified path as the last-loaded recent map in user settings.
    #[arg(long, value_name = "PATH")]
    pub recent_map: Option<PathBuf>,

    /// Override the global log level (trace, debug, info, warn, error).
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    pub log_level: String,

    /// Override the directory where subsystem log files are written.
    #[arg(long, value_name = "PATH")]
    pub log_file: Option<PathBuf>,

    /// Launch the window in borderless fullscreen mode.
    #[arg(long, default_value_t = false)]
    pub fullscreen: bool,

    /// Show an overlay with the wheel animation variables (`wheel_rotation_<n>_L/R`,
    /// `wheel_rotationspeed_<n>_l/r`) of every axle of the player's vehicle and its trailers.
    #[arg(long)]
    pub debug_wheel_vars: bool,

    /// Synchronise frames with the display (V-Sync). By default the frame rate is not capped: it
    /// is limited by the hardware only.
    #[arg(long)]
    pub vsync: bool,

    /// Rate of the fixed simulation step in Hz (physics, collisions, scripts). Rendering is
    /// independent of it and interpolated, so the frame rate stays uncapped.
    #[arg(long, value_name = "HZ", default_value_t = 64.0)]
    pub physics_hz: f64,

    /// Explicitly request windowed mode (overrides fullscreen if both present).
    #[arg(long, default_value_t = false)]
    pub windowed: bool,

    /// Initial window dimensions formatted as `WIDTHxHEIGHT` (e.g. 1920x1080); values below the
    /// minimum window size of 1024x640 are raised to it.
    #[arg(long, value_name = "WxH", default_value = "1920x1080")]
    pub window_size: String,

    /// Run the application headlessly without spawning an OS display window (ideal for CI testing).
    #[arg(long, default_value_t = false)]
    pub headless: bool,

    /// Start the control server on this TCP port (`0` = pick a free port). Off when omitted.
    #[arg(long, value_name = "PORT")]
    pub cli_port: Option<u16>,

    /// Address of the control server. Anything but a loopback address also needs the token.
    #[arg(long, value_name = "ADDR", default_value = "127.0.0.1")]
    pub cli_bind: String,

    /// Refused: the token is not read from the command line (ISS-004). Set `OPTS_CLI_TOKEN` or
    /// pass `--cli-token-file`.
    #[arg(long = "cli-token", value_name = "TOKEN", hide = true)]
    pub cli_token_argv: Option<String>,

    /// File whose first line is the control token, which clients must present (`auth`) before any
    /// other command. Alternative to `OPTS_CLI_TOKEN`; the token is required on every bind.
    #[arg(long, value_name = "PATH")]
    pub cli_token_file: Option<PathBuf>,

    /// The control token, resolved by [`CliArgs::resolve_cli_token`]; not a flag.
    #[arg(skip)]
    pub cli_token: Option<String>,

    /// Extra directory the control server may read from and write to (repeatable).
    #[arg(long, value_name = "DIR")]
    pub cli_allow_dir: Vec<PathBuf>,

    /// Repaint of the player's vehicle (a name from its `.cti` files; default: the `.bus` default).
    #[arg(long, value_name = "NAME")]
    pub livery: Option<String>,

    /// Start point of the map to begin at: its name or a part of it that only one start point
    /// has (`Ahlheim Hbf`). Default: the map's first start point.
    #[arg(long, value_name = "NAME")]
    pub entry: Option<String>,

    /// Weather preset of the session (an installed preset's id or name).
    #[arg(long, value_name = "PRESET")]
    pub weather: Option<String>,

    /// Date of the session, `YYYY-MM-DD`.
    #[arg(long, value_name = "DATE")]
    pub date: Option<String>,

    /// Time of day the session starts at, `HH:MM`.
    #[arg(long, value_name = "TIME")]
    pub time: Option<String>,

    /// An equipment option of the player's vehicle, `NAME=VALUE` (a `[setvar]` of the `.bus`;
    /// repeat for several).
    #[arg(long = "setvar", value_name = "NAME=VALUE")]
    pub setvars: Vec<String>,

    /// Number of cyclists on cycle paths (default 6; `0` = none).
    #[arg(long, value_name = "N")]
    pub cyclists: Option<usize>,

    /// Number of AI trains (default 3; `0` = none).
    #[arg(long, value_name = "N")]
    pub trains: Option<usize>,

    /// Cap on simultaneously walking pedestrians (`0` = none; default 40).
    #[arg(long, value_name = "N")]
    pub pedestrians: Option<usize>,

    /// Cap on simultaneously driving AI vehicles (`0` = none; default 30).
    #[arg(long, value_name = "N")]
    pub traffic: Option<usize>,

    /// Texture filtering for this run only, not saved: `off`, `4`, `8` or `16` (default: the setting).
    #[arg(long, value_name = "LEVEL")]
    pub texture_filter: Option<String>,

    /// Colour look for this run only, not saved: `vivid`, `natural` or `bevy` (default: the setting).
    #[arg(long, value_name = "LOOK")]
    pub look: Option<String>,

    /// Scene trace (debug): record which mesh is placed where, as JSON Lines next to the run logs.
    /// `--trace-scene` or `--trace-scene=summary` keeps counts and issues, `--trace-scene=full`
    /// every placed thing. See `docs/src/architecture/scene-trace.md`.
    #[arg(
        long,
        value_name = "MODE",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "summary"
    )]
    pub trace_scene: Option<String>,

    /// Disable the file watcher that hot-reloads edited OMSI content.
    #[arg(long, default_value_t = false)]
    pub no_hot_reload: bool,

    /// Extra directory to watch for hot reload (repeatable).
    #[arg(long, value_name = "DIR")]
    pub hot_reload_root: Vec<PathBuf>,

    /// Quiet period in milliseconds that coalesces bursts of file events.
    #[arg(long, value_name = "MS", default_value_t = 250)]
    pub hot_reload_debounce_ms: u64,
}

mod defaults;

#[cfg(test)]
mod tests;
