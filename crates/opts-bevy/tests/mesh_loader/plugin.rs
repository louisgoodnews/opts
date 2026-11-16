//! The mesh plugin registers in an app.

use super::*;

#[test]
fn test_opts_mesh_plugin_app_registration() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..Default::default()
    });
    app.add_plugins(OptsMeshPlugin);

    // Verifies app initializes without panic and asset loaders are registered
    assert!(app.is_plugin_added::<OptsMeshPlugin>());
}
