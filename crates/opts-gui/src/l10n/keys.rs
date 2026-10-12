//! The message keys used in code. Key tables may hold string literals; everything else in
//! `opts-gui` may not (see the test `no_user_visible_literals_outside_key_tables`).

/// Tooltip of a text field's clear button.
pub const CLEAR: &str = "widget-clear";
/// Default placeholder of a combobox.
pub const COMBOBOX_PLACEHOLDER: &str = "widget-combobox-placeholder";
/// The red marker text.
pub const NOT_AVAILABLE: &str = "widget-not-available-yet";
/// Tooltip text of a not-yet control (variable: `milestone`).
pub const NOT_AVAILABLE_REASON: &str = "widget-not-available-yet-reason";
/// Item count with plural (variable: `count`).
pub const ITEMS: &str = "widget-items";

/// Keys of the former main menu that the library tests still use.
pub mod menu {
    /// Shutdown button.
    pub const SHUTDOWN: &str = "menu-shutdown";
    /// Settings button.
    pub const SETTINGS: &str = "menu-settings";
}

/// Every key above; the completeness tests iterate it.
pub const ALL: &[&str] = &[
    CLEAR,
    COMBOBOX_PLACEHOLDER,
    NOT_AVAILABLE,
    NOT_AVAILABLE_REASON,
    ITEMS,
    menu::SHUTDOWN,
    menu::SETTINGS,
];
