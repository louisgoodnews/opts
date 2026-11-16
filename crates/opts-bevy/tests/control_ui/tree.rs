//! The UI tree lists ids, roles, labels and state.

use super::*;

#[test]
fn tree_lists_ids_roles_labels_and_state() {
    let (mut app, port, _) = app();
    let tree = with_client(&mut app, port, |c| {
        c.call("ui.tree", Value::Null).expect("ui.tree")
    });
    let nodes = tree["tree"].as_array().expect("tree");
    let flat: Vec<&Value> = nodes.iter().collect();
    let find = |id: &str| {
        flat.iter()
            .find(|n| n["id"] == id)
            .copied()
            .unwrap_or_else(|| panic!("no {id}"))
    };
    assert_eq!(find("menu.start")["role"], "button");
    assert_eq!(find("menu.start")["label"], "Start");
    assert_eq!(find("menu.start")["focusable"], true);
    assert_eq!(find("menu.load")["state"]["disabled"], true);
    assert_eq!(
        find("menu.load")["state"]["disabled_reason"],
        "Needs a saved game"
    );
    assert_eq!(tree["count"], 3);
}
