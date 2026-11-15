//! Collecting the scrollables of the open UI, and the items of each one.

use super::*;

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

pub(super) fn items_of(world: &mut World, s: &Scrollable) -> Vec<Value> {
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
