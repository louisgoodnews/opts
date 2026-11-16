//! Lookups of the vehicle key actions by id, setting or trigger.

use super::{VehicleKeyAction, ACTIONS};

impl VehicleKeyAction {
    /// The setting that holds this action's key for a bus (`rail`: false) or a rail vehicle.
    #[must_use]
    pub fn setting_for(&self, rail: bool) -> &'static str {
        if rail {
            self.rail_setting.unwrap_or(self.setting)
        } else {
            self.setting
        }
    }

    /// Whether `setting` is one of this action's keys.
    #[must_use]
    pub fn owns(&self, setting: &str) -> bool {
        self.setting == setting || self.rail_setting == Some(setting)
    }
}

/// The action with this id or this setting key.
#[must_use]
pub fn action(id_or_setting: &str) -> Option<&'static VehicleKeyAction> {
    ACTIONS
        .iter()
        .find(|a| a.id == id_or_setting || a.owns(id_or_setting))
}

/// The action that raises `trigger`.
#[must_use]
pub fn action_of_trigger(trigger: &str) -> Option<&'static VehicleKeyAction> {
    ACTIONS.iter().find(|a| a.trigger == trigger)
}
