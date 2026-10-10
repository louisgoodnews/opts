//! Configuration loading and validation integration tests.

use opts_logging::config::{
    AnsiMode, ConsoleVerbosity, FilesConfig, LogConfig, LogFormat, RetentionPolicy, RotationPolicy,
};
use opts_logging::error::LogError;
use std::path::PathBuf;

#[test]
fn test_parse_sample_toml() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sample_config.toml");

    let config = LogConfig::from_file(&fixture_path).expect("failed to load sample_config.toml");

    assert_eq!(config.level, "debug");
    assert_eq!(
        config.per_target.get("wgpu").map(String::as_str),
        Some("warn")
    );
    assert_eq!(
        config.per_target.get("opts_sim").map(String::as_str),
        Some("trace")
    );
    assert!(config.console.enabled);
    assert_eq!(config.console.ansi, AnsiMode::Never);
    assert_eq!(config.console.verbosity, ConsoleVerbosity::Standard);
    assert!(config.files.enabled);
    assert_eq!(config.files.rotation, RotationPolicy::Never);
    assert_eq!(config.files.retention, RetentionPolicy::Forever);
    assert!(config.files.per_subsystem);
    assert_eq!(config.format, LogFormat::Text);
}

#[test]
fn test_parse_environment_overrides() {
    // Set test env variables
    std::env::set_var("OPTS_LOG_LEVEL", "trace");
    std::env::set_var("OPTS_LOG_CONSOLE", "false");
    std::env::set_var("OPTS_LOG_DIR", "/custom/log/dir");
    std::env::set_var("OPTS_LOG_FORMAT", "json");

    let config = LogConfig::from_env();

    assert_eq!(config.level, "trace");
    assert!(!config.console.enabled);
    assert_eq!(config.files.directory, PathBuf::from("/custom/log/dir"));
    assert_eq!(config.format, LogFormat::Json);

    // Clean up
    std::env::remove_var("OPTS_LOG_LEVEL");
    std::env::remove_var("OPTS_LOG_CONSOLE");
    std::env::remove_var("OPTS_LOG_DIR");
    std::env::remove_var("OPTS_LOG_FORMAT");
}

#[test]
fn test_validate_rejects_empty_level() {
    let config = LogConfig {
        level: "   ".to_string(),
        ..Default::default()
    };

    match config.validate() {
        Err(LogError::InvalidConfig(msg)) => {
            assert!(msg.contains("level"));
        }
        _ => panic!("expected invalid config error"),
    }
}

#[test]
fn test_validate_rejects_empty_directory_when_files_enabled() {
    let config = LogConfig {
        files: FilesConfig {
            enabled: true,
            directory: PathBuf::new(),
            ..Default::default()
        },
        ..Default::default()
    };

    match config.validate() {
        Err(LogError::InvalidConfig(msg)) => {
            assert!(msg.contains("directory"));
        }
        _ => panic!("expected invalid config error"),
    }
}

#[test]
fn test_validate_rejects_zero_retention_days() {
    let config = LogConfig {
        files: FilesConfig {
            rotation: RotationPolicy::Daily,
            retention: RetentionPolicy::Days(0),
            ..Default::default()
        },
        ..Default::default()
    };

    match config.validate() {
        Err(LogError::InvalidConfig(msg)) => {
            assert!(msg.contains("retention"));
        }
        _ => panic!("expected invalid config error"),
    }
}
