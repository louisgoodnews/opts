//! The filter of a searchable combobox: applies the debounced query to the options and the
//! "No match" row, then acts on a pending Down or Enter.

use bevy::ecs::system::SystemParam;

use super::*;

/// Parameters for [`combobox_filter_system`].
#[derive(SystemParam)]
pub struct ComboboxFilterParams<'w, 's> {
    pub time: Option<Res<'w, Time>>,
    pub roots: Query<'w, 's, (Entity, &'static M3Combobox, &'static mut ComboboxFilter)>,
    pub searches: Query<'w, 's, (&'static ComboboxSearch, &'static M3TextField)>,
    pub items: Query<
        'w,
        's,
        (Entity, &'static ComboboxMenuItem, &'static mut Node),
        Without<ComboboxNoMatch>,
    >,
    pub no_match:
        Query<'w, 's, (&'static ComboboxNoMatch, &'static mut Node), Without<ComboboxMenuItem>>,
    pub focus: MessageWriter<'w, FocusEvent>,
    pub activated: MessageWriter<'w, Activated>,
}

/// Applies the search text to the options after the debounce, shows the "No match" row and acts
/// on a pending Down / Enter.
pub fn combobox_filter_system(mut params: ComboboxFilterParams) {
    let dt = params.time.map_or(0.0, |t| t.delta_secs());
    for (root, combo, mut filter) in &mut params.roots {
        let Some((_, field)) = params.searches.iter().find(|(s, _)| s.0 == root) else {
            continue;
        };
        if field.text != filter.seen {
            filter.seen.clone_from(&field.text);
            filter.wait = Some(if field.text.is_empty() {
                0.0
            } else {
                FILTER_DEBOUNCE_SECS
            });
        }
        let Some(wait) = filter.wait else { continue };
        if wait > dt {
            filter.wait = Some(wait - dt);
            continue;
        }
        let query = filter.seen.clone();
        filter.wait = None;
        filter.applied.clone_from(&query);
        let mut visible = Vec::new();
        for (entity, item, mut node) in &mut params.items {
            if item.combobox_entity != root {
                continue;
            }
            let shown = combo
                .options
                .get(item.index)
                .is_some_and(|o| matches_query(o, &query));
            let display = if shown { Display::Flex } else { Display::None };
            if node.display != display {
                node.display = display;
            }
            if shown {
                visible.push((item.index, entity));
            }
        }
        for (n, mut node) in &mut params.no_match {
            if n.0 == root {
                let display = if visible.is_empty() {
                    Display::Flex
                } else {
                    Display::None
                };
                if node.display != display {
                    node.display = display;
                }
            }
        }
        let target = visible
            .iter()
            .find(|(i, _)| Some(*i) == combo.selected)
            .or_else(|| visible.iter().min_by_key(|(i, _)| *i))
            .map(|(_, e)| *e);
        if let Some(target) = target {
            if filter.choose {
                params.activated.write(Activated(target));
            } else if filter.focus_item {
                params.focus.write(FocusEvent::Set(target));
            }
        }
        filter.choose = false;
        filter.focus_item = false;
    }
}
