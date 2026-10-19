//! Scrollable lists: finding them, listing their items and scrolling an item into view (`ui.scrollables`, `ui.scroll`).

use super::*;

/// What kind of view a [`Scrollable`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollKind {
    /// A node with `overflow: scroll` and a [`ScrollPosition`].
    Container,
    /// A windowed master list (only some rows exist as entities).
    List(Entity),
}

/// One scrollable view as the protocol sees it.
#[derive(Debug, Clone)]
pub struct Scrollable {
    /// Id for `ui.scroll target`.
    pub id: String,
    /// The node.
    pub entity: Entity,
    /// Kind.
    pub kind: ScrollKind,
    /// Rectangle in logical pixels.
    pub rect: Option<[f32; 4]>,
    /// Current offset `(x, y)` in logical pixels.
    pub offset: Vec2,
    /// Visible window size.
    pub viewport: Vec2,
    /// Content size.
    pub content: Vec2,
    /// Displayed (not hidden by an ancestor).
    pub visible: bool,
}

impl Scrollable {
    /// The largest offset.
    #[must_use]
    pub fn max(&self) -> Vec2 {
        (self.content - self.viewport).max(Vec2::ZERO)
    }
}

/// Every scrollable view, containers in layout order, then master lists.
pub fn collect_scrollables(world: &mut World) -> Vec<Scrollable> {
    let mut out: Vec<Scrollable> = Vec::new();
    let mut taken: std::collections::HashSet<String> = world
        .query::<&UiId>()
        .iter(world)
        .map(|u| u.0.to_string())
        .collect();
    let mut containers: Vec<Entity> = world
        .query::<(Entity, &Node, &ScrollPosition)>()
        .iter(world)
        .filter(|(_, n, _)| {
            n.overflow.x == OverflowAxis::Scroll || n.overflow.y == OverflowAxis::Scroll
        })
        .map(|(e, ..)| e)
        .collect();
    containers.sort();
    for e in containers {
        let own = world.get::<UiId>(e).map(|u| u.0.to_string());
        let id = match own {
            Some(id) => id,
            None => {
                let base = format!("{}/scroll", nearest_id(world, e).unwrap_or_default());
                let mut id = base.clone();
                let mut n = 2;
                while !taken.insert(id.clone()) {
                    id = format!("{base}-{n}");
                    n += 1;
                }
                id
            }
        };
        let (Some(c), Some(p)) = (world.get::<ComputedNode>(e), world.get::<ScrollPosition>(e))
        else {
            continue;
        };
        let k = c.inverse_scale_factor;
        let (viewport, content) = (c.size() * k, c.content_size() * k);
        out.push(Scrollable {
            id,
            entity: e,
            kind: ScrollKind::Container,
            rect: rect_of(world, e),
            offset: p.0,
            viewport,
            content,
            visible: is_displayed(world, e),
        });
    }
    let lists: Vec<(Entity, Entity)> = world
        .query::<(Entity, &MasterList)>()
        .iter(world)
        .map(|(e, l)| (e, l.0))
        .collect();
    for (e, root) in lists {
        let Some(view) = world.get::<MasterDetailView>(root) else {
            continue;
        };
        let id = world.get::<UiId>(e).map_or_else(
            || format!("{}/list", nearest_id(world, e).unwrap_or_default()),
            |u| u.0.to_string(),
        );
        out.push(Scrollable {
            id,
            entity: e,
            kind: ScrollKind::List(root),
            rect: rect_of(world, e),
            offset: Vec2::new(0.0, view.scroll()),
            viewport: Vec2::new(0.0, view.viewport()),
            content: Vec2::new(0.0, view.content_height()),
            visible: is_displayed(world, e),
        });
    }
    out
}

fn items_of(world: &mut World, s: &Scrollable) -> Vec<Value> {
    match s.kind {
        ScrollKind::List(root) => {
            let (Some(view), Some(md)) = (
                world.get::<MasterDetailView>(root),
                world.get::<MasterDetail>(root),
            ) else {
                return Vec::new();
            };
            view.shown_entries(md)
                .into_iter()
                .map(|(id, title)| json!({ "id": id, "title": title }))
                .collect()
        }
        ScrollKind::Container => {
            let snap = snapshot(world);
            let mut ids = Vec::new();
            for n in snap.flatten() {
                if let Some(e) = find_entity(world, &n.id) {
                    if e != s.entity && is_inside(world, e, s.entity) {
                        ids.push(json!({ "id": n.id, "role": n.role.as_str(), "label": n.label }));
                    }
                }
                if ids.len() >= 500 {
                    break;
                }
            }
            ids
        }
    }
}

/// How many items the scrollable `id` holds (`None` when there is no such view).
pub fn scrollable_item_count(world: &mut World, id: &str) -> Option<usize> {
    let s = collect_scrollables(world)
        .into_iter()
        .find(|s| s.id == id)?;
    Some(items_of(world, &s).len())
}

fn scrollable_json(world: &mut World, s: &Scrollable, with_items: bool) -> Value {
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

pub(super) fn scrollables(world: &mut World, params: &Value) -> Outcome {
    let visible_only = match opt_bool(params, "visible_only") {
        Ok(v) => v.unwrap_or(false),
        Err(e) => return Outcome::err(e),
    };
    let list = scrollables_json(world, visible_only, true);
    Outcome::ok(json!({ "count": list.len(), "scrollables": list }))
}

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

/// The view under keyboard focus, else the first visible one inside the open dialog (or the
/// whole screen), preferring containers that can actually scroll.
fn default_target(world: &mut World, all: &[Scrollable]) -> Option<Scrollable> {
    let focus = world
        .query_filtered::<Entity, With<opts_gui::interaction::state::Focused>>()
        .iter(world)
        .next();
    if let Some(f) = focus {
        if let Some(s) = all
            .iter()
            .rev()
            .find(|s| s.visible && is_inside(world, f, s.entity))
        {
            return Some(s.clone());
        }
    }
    let modals = open_modals(world);
    let scoped = |w: &World, s: &&Scrollable| {
        modals.is_empty() || modals.iter().any(|m| is_inside(w, s.entity, *m))
    };
    let usable: Vec<&Scrollable> = all
        .iter()
        .filter(|s| s.visible)
        .filter(|s| scoped(world, s))
        .collect();
    usable
        .iter()
        .find(|s| s.max().length() > 0.5)
        .or_else(|| usable.first())
        .map(|s| (*s).clone())
}

fn reveal_item(world: &mut World, target: &Scrollable, id: &str) -> Result<Vec2, RpcError> {
    match target.kind {
        ScrollKind::List(root) => {
            let md = world
                .get::<MasterDetail>(root)
                .ok_or_else(|| not_found(id))?;
            let entry = md
                .entries
                .iter()
                .find(|e| e.id == id)
                .or_else(|| md.entries.iter().find(|e| e.title.eq_ignore_ascii_case(id)))
                .ok_or_else(|| {
                    RpcError::new(
                        code::NOT_FOUND,
                        format!("`{id}` is not an entry of `{}`", target.id),
                    )
                    .with_data(json!({
                        "hint": "entry ids and titles are listed in `ui.scrollables` (`items`)",
                    }))
                })?;
            let view = world
                .get::<MasterDetailView>(root)
                .ok_or_else(|| not_found(id))?;
            let offset = view.reveal_offset(md, &entry.id).ok_or_else(|| {
                RpcError::new(
                    code::NOT_READY,
                    format!("`{id}` is filtered out of the list"),
                )
            })?;
            Ok(Vec2::new(0.0, offset))
        }
        ScrollKind::Container => {
            let entity = find_entity(world, id).ok_or_else(|| not_found(id))?;
            if !is_inside(world, entity, target.entity) {
                return Err(RpcError::new(
                    code::NOT_FOUND,
                    format!("`{id}` is not inside the view `{}`", target.id),
                ));
            }
            let (Some(item), Some(view)) = (rect_of(world, entity), target.rect) else {
                return Err(RpcError::new(
                    code::NOT_READY,
                    format!("`{id}` has no layout yet"),
                ));
            };
            let mut next = target.offset;
            if item[1] < view[1] {
                next.y -= view[1] - item[1];
            } else if item[1] + item[3] > view[1] + view[3] {
                next.y += (item[1] + item[3]) - (view[1] + view[3]);
                // Never push the item's top out of view while revealing a tall one.
                next.y = next.y.min(target.offset.y + (item[1] - view[1]));
            }
            if item[0] < view[0] {
                next.x -= view[0] - item[0];
            } else if item[0] + item[2] > view[0] + view[2] {
                next.x += (item[0] + item[2]) - (view[0] + view[2]);
                next.x = next.x.min(target.offset.x + (item[0] - view[0]));
            }
            Ok(next)
        }
    }
}
