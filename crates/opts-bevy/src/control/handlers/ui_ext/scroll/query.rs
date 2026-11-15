//! The read side of the `scroll` command: item counts and the JSON of the scrollables.

use super::*;

/// How many items the scrollable `id` holds (`None` when there is no such view).
pub fn scrollable_item_count(world: &mut World, id: &str) -> Option<usize> {
    let s = collect_scrollables(world)
        .into_iter()
        .find(|s| s.id == id)?;
    Some(items_of(world, &s).len())
}

pub(super) fn scrollable_json(world: &mut World, s: &Scrollable, with_items: bool) -> Value {
    let max = s.max();
    let mut v = json!({
        "id": s.id,
        "kind": match s.kind { ScrollKind::Container => "container", ScrollKind::List(_) => "list" },
        "rect": s.rect,
        "visible": s.visible,
        "offset": [s.offset.x, s.offset.y],
        "viewport": [s.viewport.x, s.viewport.y],
        "content": [s.content.x, s.content.y],
        "max": [max.x, max.y],
        "at_top": s.offset.y <= 0.5,
        "at_bottom": s.offset.y >= max.y - 0.5,
    });
    if with_items {
        v["items"] = Value::Array(items_of(world, s));
    }
    v
}

/// The scrollables as JSON (also used by `ui.screen`).
pub fn scrollables_json(world: &mut World, visible_only: bool, with_items: bool) -> Vec<Value> {
    collect_scrollables(world)
        .into_iter()
        .filter(|s| !visible_only || s.visible)
        .map(|s| scrollable_json(world, &s, with_items))
        .collect()
}

pub(crate) fn scrollables(world: &mut World, params: &Value) -> Outcome {
    let visible_only = match opt_bool(params, "visible_only") {
        Ok(v) => v.unwrap_or(false),
        Err(e) => return Outcome::err(e),
    };
    let list = scrollables_json(world, visible_only, true);
    Outcome::ok(json!({ "count": list.len(), "scrollables": list }))
}
