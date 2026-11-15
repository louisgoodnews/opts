//! Scrolls the selected option of an open combobox menu into view.

use super::*;

/// Scrolls the selected option into view a frame after the menu opened (when it has a layout).
pub fn combobox_reveal_system(
    mut commands: Commands,
    mut roots: Query<(Entity, &M3Combobox, &mut ComboboxReveal)>,
    items: Query<(Entity, &ComboboxMenuItem)>,
    parents: Query<&ChildOf>,
    geometry: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut lists: Query<(Entity, &ComboboxList, &mut ScrollPosition)>,
) {
    for (root, combo, mut reveal) in &mut roots {
        if !combo.is_open {
            commands.entity(root).try_remove::<ComboboxReveal>();
            continue;
        }
        if reveal.0 < 1 {
            reveal.0 += 1;
            continue;
        }
        commands.entity(root).try_remove::<ComboboxReveal>();
        let Some(selected) = combo.selected else {
            continue;
        };
        let Some((item, _)) = items
            .iter()
            .find(|(_, i)| i.combobox_entity == root && i.index == selected)
        else {
            continue;
        };
        let Some((list, _, mut position)) = lists.iter_mut().find(|(_, l, _)| l.0 == root) else {
            continue;
        };
        if parents.get(item).map(ChildOf::parent) != Ok(list) {
            continue;
        }
        let (Ok((ic, it)), Ok((lc, lt))) = (geometry.get(item), geometry.get(list)) else {
            continue;
        };
        let rect = |c: &ComputedNode, t: &UiGlobalTransform| {
            let k = c.inverse_scale_factor;
            let half = c.size() / 2.0;
            ((t.translation - half) * k, (t.translation + half) * k)
        };
        let (imin, imax) = rect(ic, it);
        let (lmin, lmax) = rect(lc, lt);
        if imax.y - imin.y <= 0.0 || lmax.y - lmin.y <= 0.0 {
            continue;
        }
        if imin.y < lmin.y || imax.y > lmax.y {
            // Centre it when there is room.
            let slack = ((lmax.y - lmin.y) - (imax.y - imin.y)).max(0.0) / 2.0;
            position.y = (position.y + (imin.y - lmin.y) - slack).max(0.0);
        }
    }
}
