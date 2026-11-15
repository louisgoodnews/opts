//! The combobox search: filter, keys, reveal and no-match row.

use super::*;

mod filter;
mod reveal;

pub use filter::*;
pub use reveal::*;

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
