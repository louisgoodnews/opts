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

/// Names of the theme choices (settings page, theme previews).
pub mod theme {
    /// Theme mode: dark.
    pub const MODE_DARK: &str = "theme-mode-dark";
    /// Theme mode: light.
    pub const MODE_LIGHT: &str = "theme-mode-light";
    /// Theme mode: follow the desktop.
    pub const MODE_SYSTEM: &str = "theme-mode-system";
    /// Theme mode: by the clock.
    pub const MODE_AUTO: &str = "theme-mode-auto";
    /// The high-contrast theme.
    pub const HIGH_CONTRAST: &str = "theme-high-contrast";
    /// Accent preset: Transit blue.
    pub const ACCENT_DEFAULT: &str = "theme-accent-default";
    /// Accent preset: Amber.
    pub const ACCENT_AMBER: &str = "theme-accent-amber";
    /// Accent preset: Violet.
    pub const ACCENT_VIOLET: &str = "theme-accent-violet";
    /// Accent preset: Emerald.
    pub const ACCENT_EMERALD: &str = "theme-accent-emerald";
    /// Accent preset: Teal.
    pub const ACCENT_TEAL: &str = "theme-accent-teal";
    /// Density: comfortable.
    pub const DENSITY_COMFORTABLE: &str = "theme-density-comfortable";
    /// Density: compact.
    pub const DENSITY_COMPACT: &str = "theme-density-compact";
    /// Density: dense.
    pub const DENSITY_DENSE: &str = "theme-density-dense";
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
    theme::MODE_DARK,
    theme::MODE_LIGHT,
    theme::MODE_SYSTEM,
    theme::MODE_AUTO,
    theme::HIGH_CONTRAST,
    theme::ACCENT_DEFAULT,
    theme::ACCENT_AMBER,
    theme::ACCENT_VIOLET,
    theme::ACCENT_EMERALD,
    theme::ACCENT_TEAL,
    theme::DENSITY_COMFORTABLE,
    theme::DENSITY_COMPACT,
    theme::DENSITY_DENSE,
];
