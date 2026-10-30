//! The *Run* and category buttons of the settings page.

use super::*;

/// A *Run* button: a key binding starts (or cancels) waiting for its key, any other setting is
/// an action for whoever owns it.
pub(super) fn run_op(
    key: &'static str,
    state: &mut SettingsPageState,
    api: &SettingsApi,
    actions: &mut MessageWriter<SettingActionRequested>,
    loc: &opts_gui::l10n::Localization,
) {
    if api
        .registry
        .get(key)
        .is_some_and(|d| matches!(d.kind, SettingKind::Binding))
    {
        // A key binding: the next key press is the new key (`capture_rebind`).
        state.rebinding = if state.rebinding == Some(key) {
            None
        } else {
            Some(key)
        };
        state.notice = state.rebinding.map(|_| loc.t("settings-rebind-prompt"));
        state.dirty = true;
    } else {
        actions.write(SettingActionRequested { key });
    }
}

/// A category button: switch to it and clear the search.
pub(super) fn select_category(state: &mut SettingsPageState, cat: Category) {
    if state.category != cat || !state.query.is_empty() {
        state.category = cat;
        state.query.clear();
        state.clear_search_field = true;
        state.dirty = true;
    }
}
