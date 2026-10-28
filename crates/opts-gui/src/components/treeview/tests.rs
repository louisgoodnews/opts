//! Unit tests for `super` (`treeview.rs`).

use super::*;

#[test]
fn test_treeview_builder_spawn() {
    let mut app = App::new();
    let entity = {
        let mut commands = app.world_mut().commands();
        let root_node =
            TreeNode::new("root", "Root Node").with_child(TreeNode::new("child1", "Child Node"));

        TreeviewBuilder::new()
            .with_node(root_node)
            .spawn(&mut commands)
    };

    app.update();

    let tree = app.world().get::<M3Treeview>(entity);
    assert!(tree.is_some());
}
