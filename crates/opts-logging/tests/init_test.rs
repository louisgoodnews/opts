//! Initialization lifecycle integration tests.

use opts_logging::config::{ConsoleConfig, FilesConfig, LogConfig};
use opts_logging::error::LogError;

#[test]
fn test_init_succeeds_once_and_fails_twice() {
    let config = LogConfig {
        console: ConsoleConfig {
            enabled: false,
            ..Default::default()
        },
        files: FilesConfig {
            enabled: false,
            ..Default::default()
        },
        ..Default::default()
    };

    // First init should succeed
    let guard1 = opts_logging::init(config.clone());
    assert!(guard1.is_ok(), "first initialization should succeed");

    // Second init must fail with AlreadyInitialized
    let guard2 = opts_logging::init(config);
    match guard2 {
        Err(LogError::AlreadyInitialized) => {
            // Expected behavior
        }
        other => panic!("expected AlreadyInitialized, got {other:?}"),
    }
}
