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
