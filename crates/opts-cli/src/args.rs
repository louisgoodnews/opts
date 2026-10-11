//! Command-line arguments of the game binary (`opts`), including the control-server and hot-reload switches.

use clap::Parser;
use std::path::PathBuf;

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

    /// Explicitly request windowed mode (overrides fullscreen if both present).
    #[arg(long, default_value_t = false)]
    pub windowed: bool,

    /// Initial window dimensions formatted as `WIDTHxHEIGHT` (e.g. 1920x1080).
    #[arg(long, value_name = "WxH", default_value = "1280x720")]
    pub window_size: String,

    /// Run the application headlessly without spawning an OS display window (ideal for CI testing).
    #[arg(long, default_value_t = false)]
    pub headless: bool,

    /// Start the control server on this TCP port (`0` = pick a free port). Off when omitted.
    #[arg(long, value_name = "PORT")]
    pub cli_port: Option<u16>,

    /// Address of the control server. Anything but a loopback address requires `--cli-token`.
    #[arg(long, value_name = "ADDR", default_value = "127.0.0.1")]
    pub cli_bind: String,

    /// Shared secret the control clients must present (`auth`) before any other command.
    #[arg(long, value_name = "TOKEN")]
    pub cli_token: Option<String>,

    /// Extra directory the control server may read from and write to (repeatable).
    #[arg(long, value_name = "DIR")]
    pub cli_allow_dir: Vec<PathBuf>,

    /// Repaint of the player's vehicle (a name from its `.cti` files; default: the `.bus` default).
    #[arg(long, value_name = "NAME")]
    pub livery: Option<String>,

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

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            map_path: None,
            omsi_path: None,
            vehicle_path: None,
            autostart: false,
            camera: None,
            screenshot: None,
            screenshot_delay: 4.0,
            recent_map: None,
            log_level: "info".to_string(),
            log_file: None,
            fullscreen: false,
            windowed: false,
            window_size: "1280x720".to_string(),
            headless: false,
            cli_port: None,
            cli_bind: "127.0.0.1".to_string(),
            cli_token: None,
            pedestrians: None,
            trains: None,
            cyclists: None,
            livery: None,
            traffic: None,
            cli_allow_dir: Vec::new(),
            no_hot_reload: false,
            hot_reload_root: Vec::new(),
            hot_reload_debounce_ms: 250,
        }
    }
}

impl CliArgs {
    /// Parses arguments from the process environment (`std::env::args`).
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Parses window resolution width and height from the `--window-size` flag.
    /// Falls back to 1280x720 if invalid.
    pub fn parse_dimensions(&self) -> (f32, f32) {
        let parts: Vec<&str> = self.window_size.split('x').collect();
        if parts.len() == 2 {
            if let (Ok(w), Ok(h)) = (
                parts[0].trim().parse::<f32>(),
                parts[1].trim().parse::<f32>(),
            ) {
                if w >= 400.0 && h >= 300.0 {
                    return (w, h);
                }
            }
        }
        (1280.0, 720.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_custom_flags() {
        let args = CliArgs::try_parse_from([
            "opts",
            "--map-path",
            "/home/user/maps/Berlin-Spandau",
            "--log-level",
            "debug",
            "--window-size",
            "1920x1080",
            "--fullscreen",
            "--recent-map",
            "/home/user/maps/Berlin-Spandau",
        ])
        .expect("CLI arguments should parse successfully");

        assert_eq!(
            args.map_path,
            Some(PathBuf::from("/home/user/maps/Berlin-Spandau"))
        );
        assert_eq!(
            args.recent_map,
            Some(PathBuf::from("/home/user/maps/Berlin-Spandau"))
        );
        assert_eq!(args.log_level, "debug");
        assert_eq!(args.window_size, "1920x1080");
        assert!(args.fullscreen);
        assert!(!args.headless);
        assert_eq!(args.parse_dimensions(), (1920.0, 1080.0));
    }

    #[test]
    fn control_flags_parse_and_default_off() {
        let args = CliArgs::try_parse_from(["opts"]).expect("defaults");
        assert_eq!(args.cli_port, None);
        assert_eq!(args.cli_bind, "127.0.0.1");
        assert_eq!(args.hot_reload_debounce_ms, 250);
        let args = CliArgs::try_parse_from([
            "opts",
            "--cli-port",
            "0",
            "--cli-allow-dir",
            "/tmp/a",
            "--cli-allow-dir",
            "/tmp/b",
            "--no-hot-reload",
        ])
        .expect("flags");
        assert_eq!(args.cli_port, Some(0));
        assert_eq!(args.cli_allow_dir.len(), 2);
        assert!(args.no_hot_reload);
    }

    #[test]
    fn test_cli_parsing_defaults() {
        let args = CliArgs::try_parse_from(["opts"]).expect("Defaults should parse");

        assert_eq!(args.log_level, "info");
        assert_eq!(args.window_size, "1280x720");
        assert!(!args.headless);
        assert!(!args.fullscreen);
        assert_eq!(args.parse_dimensions(), (1280.0, 720.0));
    }
}
