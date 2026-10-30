//! System-parameter bundles of the settings page's input systems.

use bevy::ecs::system::SystemParam;

use super::*;

/// The messages the page reacts to, and the one it sends.
#[derive(SystemParam)]
pub struct PageEvents<'w, 's> {
    /// Button clicks.
    pub clicks: MessageReader<'w, 's, ButtonClicked>,
    /// Switch toggles.
    pub toggles: MessageReader<'w, 's, SwitchToggled>,
    /// Text changes (the search field).
    pub texts: MessageReader<'w, 's, TextChanged>,
    /// Combo box choices.
    pub chosen: MessageReader<'w, 's, ComboboxSelected>,
    /// Actions and choosers the page asks for.
    pub actions: MessageWriter<'w, SettingActionRequested>,
}

/// The controls the page looks up.
#[derive(SystemParam)]
pub struct PageControls<'w, 's> {
    /// Combo boxes.
    pub combos: Query<'w, 's, &'static SettingCombo>,
    /// Operations.
    pub ops: Query<'w, 's, &'static SettingOp>,
    /// Switches.
    pub switches: Query<'w, 's, &'static SettingSwitch>,
    /// The search field.
    pub search: Query<'w, 's, (), With<SettingsSearch>>,
}

/// What ends the revert prompt besides the countdown.
#[derive(SystemParam)]
pub struct RevertInput<'w, 's> {
    /// Button clicks.
    pub clicks: MessageReader<'w, 's, ButtonClicked>,
    /// Modal events (Escape).
    pub modal_events: MessageReader<'w, 's, ModalEvent>,
    /// The prompt's buttons.
    pub buttons: Query<'w, 's, &'static RevertButton>,
}
