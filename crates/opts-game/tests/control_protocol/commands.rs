//! The command table against the registry, and the `help` / `commands` schema answers.

use super::*;
use opts_cli::commands::COMMANDS;

/// Descriptors of commands that the table promises for later and no build implements yet.
const NOT_IMPLEMENTED: &[&str] = &[
    "auth",
    "entity.list",
    "entity.get",
    "resource.list",
    "resource.get",
    "asset.list",
    "asset.get",
    "entity.spawn",
    "entity.despawn",
    "entity.set_component",
    "debug.wireframe",
    "debug.normals",
    "debug.stats",
];

#[test]
fn the_command_table_and_the_registry_agree() {
    let app = base_app(Vec::new());
    let registry = app.world().resource::<ControlRegistry>();
    let registered = registry.methods();
    for m in &registered {
        assert!(
            COMMANDS.iter().any(|c| c.method == *m),
            "`{m}` is registered but has no descriptor in crates/opts-cli/src/commands.rs"
        );
    }
    for c in COMMANDS {
        let implemented = registered.contains(&c.method);
        let promised_only = NOT_IMPLEMENTED.contains(&c.method);
        assert!(
            implemented != promised_only,
            "`{}`: implemented={implemented}, listed as not implemented={promised_only}",
            c.method
        );
    }
}

#[test]
fn help_and_commands_describe_the_schema() {
    let mut app = base_app(Vec::new());
    let h = call(&mut app, "help", json!({"command": "drive.set"})).expect("help");
    assert_eq!(h["since"], "1.1");
    assert_eq!(h["implemented"], true);
    assert_eq!(h["args"][0]["name"], "control");
    assert_eq!(h["args"][1]["name"], "value");
    let all = call(&mut app, "commands", json!({"group": "drive"})).expect("commands");
    assert_eq!(all["protocol_version"], "1.31.0");
    let names: Vec<&str> = all["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["method"].as_str())
        .collect();
    for want in ["drive.set", "drive.trigger", "drive.state", "vehicle.reset"] {
        assert!(names.contains(&want), "{want} in {names:?}");
    }
    assert!(all["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["name"] == "GATED"));
    let none = call(&mut app, "commands", json!({"group": "nonsense"})).expect_err("group");
    assert_eq!(none.code, code::NOT_FOUND);
    assert!(none.data.expect("data")["groups"].is_array());
}
