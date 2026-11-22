//! Defaults of the command-line arguments and the parsing helpers of [`CliArgs`].

use clap::{CommandFactory, Parser};

use super::{CliArgs, MIN_WINDOW_SIZE};

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            map_path: None,
            omsi_path: None,
            vehicle_path: None,
            autostart: false,
            accept_load_problems: false,
            camera: None,
            screenshot: None,
            screenshot_delay: 4.0,
            recent_map: None,
            log_level: "info".to_string(),
            log_file: None,
            fullscreen: false,
            windowed: false,
            vsync: false,
            physics_hz: 64.0,
            debug_wheel_vars: false,
            window_size: "1920x1080".to_string(),
            headless: false,
            cli_port: None,
            cli_bind: "127.0.0.1".to_string(),
            cli_token_argv: None,
            cli_token_file: None,
            cli_token: None,
            pedestrians: None,
            trains: None,
            cyclists: None,
            livery: None,
            entry: None,
            weather: None,
            date: None,
            time: None,
            setvars: Vec::new(),
            traffic: None,
            cli_allow_dir: Vec::new(),
            texture_filter: None,
            look: None,
            trace_scene: None,
            no_hot_reload: false,
            hot_reload_root: Vec::new(),
            hot_reload_debounce_ms: 250,
        }
    }
}

impl CliArgs {
    /// Parses arguments from the process environment (`std::env::args`).
    pub fn parse_args() -> Self {
        let mut args = Self::parse();
        if let Err(e) = args.resolve_cli_token() {
            Self::command()
                .error(clap::error::ErrorKind::ArgumentConflict, e)
                .exit();
        }
        args
    }

    /// Refuses the old `--cli-token` flag and reads the token from `OPTS_CLI_TOKEN` or the file
    /// given by `--cli-token-file` into [`CliArgs::cli_token`].
    ///
    /// # Errors
    /// `--cli-token` was given, both sources are set, the file cannot be read, or `--cli-port`
    /// is given without a token.
    pub fn resolve_cli_token(&mut self) -> Result<(), String> {
        if self.cli_token_argv.is_some() {
            return Err(
                "--cli-token is refused: the token is not read from the command line. \
                 Set OPTS_CLI_TOKEN or pass --cli-token-file <PATH>"
                    .into(),
            );
        }
        self.cli_token = crate::token::from_sources(self.cli_token_file.as_deref())?;
        if self.cli_port.is_some() && self.cli_token.is_none() {
            return Err(
                "--cli-port needs a token: set OPTS_CLI_TOKEN or pass --cli-token-file <PATH>"
                    .into(),
            );
        }
        Ok(())
    }

    /// Parses window resolution width and height from the `--window-size` flag.
    /// Falls back to 1920x1080 if invalid; a valid size below [`MIN_WINDOW_SIZE`] is raised to it.
    pub fn parse_dimensions(&self) -> (f32, f32) {
        let parts: Vec<&str> = self.window_size.split('x').collect();
        if parts.len() == 2 {
            if let (Ok(w), Ok(h)) = (
                parts[0].trim().parse::<f32>(),
                parts[1].trim().parse::<f32>(),
            ) {
                if w >= 400.0 && h >= 300.0 {
                    return (w.max(MIN_WINDOW_SIZE.0), h.max(MIN_WINDOW_SIZE.1));
                }
            }
        }
        (1920.0, 1080.0)
    }
}
