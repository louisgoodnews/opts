//! The combobox search: filter, keys, reveal and no-match row.

use super::*;

/// Menus with more options than this get a search field (see [`ComboboxBuilder::with_search_threshold`]).
pub const DEFAULT_SEARCH_THRESHOLD: usize = 8;

/// How long the filter waits after the last keystroke.
pub const FILTER_DEBOUNCE_SECS: f32 = 0.15;

/// The search field of a long menu (on the text field frame); points at the combobox root.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxSearch(pub Entity);

/// The scrolling list of options inside the menu (the search field stays above it).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxList(pub Entity);

/// The "No match" row of a searchable menu.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxNoMatch(pub Entity);

/// Search state of a searchable combobox (on the root).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ComboboxFilter {
    /// The query the options are currently filtered by (after the debounce).
    pub applied: String,
    /// The last text seen in the search field.
    pub seen: String,
    /// Seconds until `seen` is applied; `None` when nothing is waiting.
    pub wait: Option<f32>,
    /// After applying, move focus onto the first (or the selected) visible option.
    pub focus_item: bool,
    /// After applying, choose the first (or the selected) visible option.
    pub choose: bool,
}

/// Gives the search field its localised accessible name and placeholder, and the search input
/// the vertical arrows (Down moves into the list, see [`combobox_search_keys_system`]).
pub fn combobox_search_text_system(
    loc: Option<Res<Localization>>,
    mut fields: Query<(&mut M3TextField, &FocusProxy, Ref<ComboboxSearch>)>,
    mut commands: Commands,
) {
    let loc_changed = loc.as_ref().is_some_and(|l| l.is_changed());
    for (mut field, proxy, search) in &mut fields {
        if search.is_added() {
            commands.entity(proxy.0).try_insert(OwnsVerticalArrows);
        }
        if !(search.is_added() || loc_changed) {
            continue;
        }
        if let Some(loc) = loc.as_ref() {
            field.label = loc.t(crate::l10n::keys::COMBOBOX_SEARCH_LABEL);
            field.placeholder = loc.t(crate::l10n::keys::COMBOBOX_SEARCH);
        }
    }
}

/// Down in the search field moves into the (filtered) list; Enter chooses the selected option if
/// it still matches, else the first match. Both wait for the filter to be applied first.
pub fn combobox_search_keys_system(
    mut actions: MessageReader<UiAction>,
    focused: Query<&TextFieldInput, With<Focused>>,
    searches: Query<&ComboboxSearch>,
    mut filters: Query<&mut ComboboxFilter>,
) {
    let mut request = |frame: Entity, choose: bool| {
        if let Ok(search) = searches.get(frame) {
            if let Ok(mut f) = filters.get_mut(search.0) {
                f.wait = Some(0.0);
                if choose {
                    f.choose = true;
                } else {
                    f.focus_item = true;
                }
            }
        }
    };
    for action in actions.read() {
        let choose = match action {
            UiAction::Navigate(Direction::Down) => false,
            UiAction::Confirm => true,
            _ => continue,
        };
        if let Some(input) = focused.iter().next() {
            request(input.0, choose);
        }
    }
}

/// Applies the search text to the options after the debounce, shows the "No match" row and acts
/// on a pending Down / Enter.
#[allow(clippy::too_many_arguments)]
pub fn combobox_filter_system(
    time: Option<Res<Time>>,
    mut roots: Query<(Entity, &M3Combobox, &mut ComboboxFilter)>,
    searches: Query<(&ComboboxSearch, &M3TextField)>,
    mut items: Query<(Entity, &ComboboxMenuItem, &mut Node), Without<ComboboxNoMatch>>,
    mut no_match: Query<(&ComboboxNoMatch, &mut Node), Without<ComboboxMenuItem>>,
    mut focus: MessageWriter<FocusEvent>,
    mut activated: MessageWriter<Activated>,
) {
    let dt = time.map_or(0.0, |t| t.delta_secs());
    for (root, combo, mut filter) in &mut roots {
        let Some((_, field)) = searches.iter().find(|(s, _)| s.0 == root) else {
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
        for (entity, item, mut node) in &mut items {
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
        for (n, mut node) in &mut no_match {
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
                activated.write(Activated(target));
            } else if filter.focus_item {
                focus.write(FocusEvent::Set(target));
            }
        }
        filter.choose = false;
        filter.focus_item = false;
    }
}

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

/// The systems of the search field of long menus.
pub fn combobox_search_systems() -> impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, ()>
{
    (
        combobox_search_text_system,
        combobox_search_keys_system,
        combobox_filter_system,
        combobox_reveal_system,
    )
        .chain()
}
