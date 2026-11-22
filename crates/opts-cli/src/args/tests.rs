//! Unit tests for command-line argument parsing: defaults, custom flags, control flags and window minimums.
use super::*;

#[test]
fn small_windows_are_raised_to_the_minimum() {
    let args = CliArgs::try_parse_from(["opts", "--window-size", "800x600"]).expect("parse");
    assert_eq!(args.parse_dimensions(), MIN_WINDOW_SIZE);
    let args = CliArgs::try_parse_from(["opts", "--window-size", "1280x720"]).expect("parse");
    assert_eq!(args.parse_dimensions(), (1280.0, 720.0));
}

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
    assert_eq!(args.window_size, "1920x1080");
    assert!(!args.headless);
    assert!(!args.fullscreen);
    assert_eq!(args.parse_dimensions(), (1920.0, 1080.0));
}

#[test]
fn the_old_cli_token_flag_is_refused_with_a_clear_error() {
    let mut args = CliArgs::try_parse_from(["opts", "--cli-port", "0", "--cli-token=secret"])
        .expect("the flag still parses, so that it can be refused");
    let err = args.resolve_cli_token().expect_err("argv token refused");
    assert!(err.contains("--cli-token is refused"), "{err}");
    assert_eq!(args.cli_token, None);
}

#[test]
fn a_missing_token_file_is_an_error() {
    let mut args = CliArgs::try_parse_from([
        "opts",
        "--cli-token-file",
        "/nonexistent/opts-token-for-tests",
    ])
    .expect("parse");
    assert!(args.resolve_cli_token().is_err());
}

#[test]
fn a_control_port_without_a_token_is_refused_at_start() {
    let mut args = CliArgs::try_parse_from(["opts", "--cli-port", "0"]).expect("parse");
    if std::env::var_os(crate::token::TOKEN_ENV).is_some() {
        return; // the test environment supplies a token: nothing to refuse
    }
    let err = args.resolve_cli_token().expect_err("no token");
    assert!(err.contains("needs a token"), "{err}");
}
