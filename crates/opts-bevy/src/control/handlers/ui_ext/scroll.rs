//! Scrollable lists: finding them, listing their items and scrolling an item into view (`ui.scrollables`, `ui.scroll`).

use super::*;

mod types;
pub use types::*;

mod collect;
pub use collect::*;

mod query;
pub use query::*;

mod reveal;
use reveal::*;


pub(super) fn scroll(world: &mut World, params: &Value) -> Outcome {
    match scroll_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn scroll_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let by = opt_f64(params, "by")?;
    let dx = opt_f64(params, "dx")?;
    let pages = opt_f64(params, "pages")?;
    let to = opt_str(params, "to")?;
    let item = opt_str(params, "item")?;
    let chosen = [
        by.is_some(),
        dx.is_some(),
        pages.is_some(),
        to.is_some(),
        item.is_some(),
    ]
    .iter()
    .filter(|b| **b)
    .count();
    if chosen != 1 {
        return Err(RpcError::invalid_params(
            "give exactly one of `by`, `dx`, `pages`, `to` or `item`",
        ));
    }
    if let Some(t) = to {
        if !["top", "bottom", "left", "right"].contains(&t) {
            return Err(
                RpcError::invalid_params("`to` is top, bottom, left or right")
                    .with_data(json!({ "valid": ["top", "bottom", "left", "right"] })),
            );
        }
    }
    let all = collect_scrollables(world);
    let target = match opt_str(params, "target")? {
        Some(id) => all.iter().find(|s| s.id == id).cloned().ok_or_else(|| {
            RpcError::new(code::NOT_FOUND, format!("no scrollable view `{id}`")).with_data(
                json!({ "valid": all.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
                                   "hint": "`ui.scrollables` lists the views" }),
            )
        })?,
        None => default_target(world, &all).ok_or_else(|| {
            RpcError::new(code::NOT_READY, "there is no scrollable view on screen")
        })?,
    };
    check_modal_scope(world, target.entity, "ui.scroll")?;
    let max = target.max();
    let mut next = target.offset;
    if let Some(by) = by {
        next.y += by as f32;
    } else if let Some(dx) = dx {
        next.x += dx as f32;
    } else if let Some(p) = pages {
        next.y += p as f32 * target.viewport.y;
    } else if let Some(t) = to {
        match t {
            "top" => next.y = 0.0,
            "bottom" => next.y = max.y,
            "left" => next.x = 0.0,
            _ => next.x = max.x,
        }
    } else if let Some(id) = item {
        next = reveal_item(world, &target, id)?;
    }
    next = next.clamp(Vec2::ZERO, max);
    match target.kind {
        ScrollKind::Container => {
            if let Some(mut p) = world.get_mut::<ScrollPosition>(target.entity) {
                p.0 = next;
            }
        }
        ScrollKind::List(root) => {
            if let Some(mut view) = world.get_mut::<MasterDetailView>(root) {
                view.scroll_to(next.y);
            }
        }
    }
    let mut after = target.clone();
    after.offset = next;
    let mut v = scrollable_json(world, &after, false);
    v["moved"] = json!([next.x - target.offset.x, next.y - target.offset.y]);
    Ok(v)
}
