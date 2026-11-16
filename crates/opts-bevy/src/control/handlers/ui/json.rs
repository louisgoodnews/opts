//! The UI tree as JSON: each node and the pruned snapshot.

use super::*;

/// JSON form of one node (also the result of `ui.state`).
pub fn node_json(n: &UiNode, with_children: bool) -> Value {
    let s = &n.state;
    let mut v = json!({
        "id": n.id,
        "role": n.role.as_str(),
        "label": n.label,
        "state": {
            "hovered": s.hovered,
            "focused": s.focused,
            "pressed": s.pressed,
            "selected": s.selected,
            "checked": s.checked,
            "expanded": s.expanded,
            "disabled": s.disabled.is_some(),
            "disabled_reason": s.disabled,
            "error": s.error,
            "visible": s.visible,
        },
        "rect": n.rect,
        "focusable": n.focusable,
    });
    if let Some(value) = &n.value {
        v["value"] = json!(value);
    }
    if with_children {
        v["children"] = Value::Array(n.children.iter().map(|c| node_json(c, true)).collect());
    }
    v
}

pub(super) fn snapshot_json(snap: &UiSnapshot, visible_only: bool) -> Value {
    fn prune(n: &UiNode) -> Option<UiNode> {
        if !n.state.visible {
            return None;
        }
        let mut n = n.clone();
        n.children = n.children.iter().filter_map(prune).collect();
        Some(n)
    }
    let roots: Vec<UiNode> = if visible_only {
        snap.roots.iter().filter_map(prune).collect()
    } else {
        snap.roots.clone()
    };
    let count: usize = UiSnapshot {
        roots: roots.clone(),
        focus: None,
    }
    .flatten()
    .len();
    json!({
        "focus": snap.focus,
        "count": count,
        "tree": roots.iter().map(|n| node_json(n, true)).collect::<Vec<_>>(),
    })
}
