//! Unit tests for `super` (`notebook.rs`).

use super::*;

#[test]
fn test_notebook_builder_spawn() {
    let mut app = App::new();
    let entity = {
        let mut commands = app.world_mut().commands();
        NotebookBuilder::new()
            .with_tabs(vec!["Tab 0".into(), "Tab 1".into()])
            .with_active(1)
            .spawn(&mut commands)
    };

    app.update();

    let nb = app.world().get::<M3Notebook>(entity);
    assert_eq!(nb.map(|n| n.active_tab), Some(1));
    assert_eq!(nb.map(|n| n.tab_count), Some(2));
}
