//! Choosing the default target of a scroll and bringing an item into view.

use super::*;

/// The view under keyboard focus, else the first visible one inside the open dialog (or the
/// whole screen), preferring containers that can actually scroll.
pub(super) fn default_target(world: &mut World, all: &[Scrollable]) -> Option<Scrollable> {
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

pub(super) fn reveal_item(
    world: &mut World,
    target: &Scrollable,
    id: &str,
) -> Result<Vec2, RpcError> {
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
