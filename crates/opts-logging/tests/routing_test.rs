//! Subsystem routing integration test verifying per-file event isolation.

use opts_logging::config::{ConsoleConfig, FilesConfig, LogConfig, RotationPolicy};
use std::fs;
use std::time::Duration;

#[test]
fn test_subsystem_routing_isolation() {
    let temp_dir =
        std::env::temp_dir().join(format!("opts_log_routing_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("failed to create temp test log dir");

    let config = LogConfig {
        level: "trace".to_string(),
        console: ConsoleConfig {
            enabled: false,
            ..Default::default()
        },
        files: FilesConfig {
            enabled: true,
            directory: temp_dir.clone(),
            rotation: RotationPolicy::Never,
            per_subsystem: true,
            ..Default::default()
        },
        ..Default::default()
    };

    // Initialize logging
    let guard = opts_logging::init(config).expect("init should succeed");

    // Emit distinct events
    tracing::info!(target: "opts_core::parser", "EVENT_OPTS_CORE_PARSER_ACTIVE");
    tracing::info!(target: "opts_sim::drivetrain", "EVENT_OPTS_SIM_TORQUE_ACTIVE");
    tracing::info!(target: "opts_gui::widget", "EVENT_OPTS_GUI_BUTTON_CLICKED");

    // Sleep slightly to let worker channels receive before drop
    std::thread::sleep(Duration::from_millis(50));

    // Explicitly drop guard to flush all non-blocking workers
    drop(guard);

    // Read log contents
    let opts_log = fs::read_to_string(temp_dir.join("opts.log")).unwrap_or_default();
    let core_log = fs::read_to_string(temp_dir.join("opts-core.log")).unwrap_or_default();
    let sim_log = fs::read_to_string(temp_dir.join("opts-sim.log")).unwrap_or_default();
    let gui_log = fs::read_to_string(temp_dir.join("opts-gui.log")).unwrap_or_default();

    // Verify aggregate file contains all events
    assert!(
        opts_log.contains("EVENT_OPTS_CORE_PARSER_ACTIVE"),
        "opts.log should contain opts_core events"
    );
    assert!(
        opts_log.contains("EVENT_OPTS_SIM_TORQUE_ACTIVE"),
        "opts.log should contain opts_sim events"
    );
    assert!(
        opts_log.contains("EVENT_OPTS_GUI_BUTTON_CLICKED"),
        "opts.log should contain opts_gui events"
    );

    // Verify opts-core.log contains ONLY opts_core events
    assert!(
        core_log.contains("EVENT_OPTS_CORE_PARSER_ACTIVE"),
        "opts-core.log must receive opts_core events"
    );
    assert!(
        !core_log.contains("EVENT_OPTS_SIM_TORQUE_ACTIVE"),
        "opts-core.log must NOT receive opts_sim events"
    );
    assert!(
        !core_log.contains("EVENT_OPTS_GUI_BUTTON_CLICKED"),
        "opts-core.log must NOT receive opts_gui events"
    );

    // Verify opts-sim.log contains ONLY opts_sim events
    assert!(
        sim_log.contains("EVENT_OPTS_SIM_TORQUE_ACTIVE"),
        "opts-sim.log must receive opts_sim events"
    );
    assert!(
        !sim_log.contains("EVENT_OPTS_CORE_PARSER_ACTIVE"),
        "opts-sim.log must NOT receive opts_core events"
    );

    // Verify opts-gui.log contains ONLY opts_gui events
    assert!(
        gui_log.contains("EVENT_OPTS_GUI_BUTTON_CLICKED"),
        "opts-gui.log must receive opts_gui events"
    );
    assert!(
        !gui_log.contains("EVENT_OPTS_CORE_PARSER_ACTIVE"),
        "opts-gui.log must NOT receive opts_core events"
    );

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
