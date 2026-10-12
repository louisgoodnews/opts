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

/// Main menu keys (the main menu is the first screen converted).
pub mod menu {
    /// Shutdown button.
    pub const SHUTDOWN: &str = "menu-shutdown";
    /// Load Last Game button.
    pub const LOAD_LAST_GAME: &str = "menu-load-last-game";
    /// Start New Game button.
    pub const START_NEW_GAME: &str = "menu-start-new-game";
    /// Settings button.
    pub const SETTINGS: &str = "menu-settings";
    /// Credits button.
    pub const CREDITS: &str = "menu-credits";
    /// Theme toggle, offers the light theme.
    pub const THEME_LIGHT: &str = "menu-theme-light";
    /// Theme toggle, offers the dark theme.
    pub const THEME_DARK: &str = "menu-theme-dark";
    /// Big title.
    pub const TITLE: &str = "menu-title";
    /// Subtitle.
    pub const SUBTITLE: &str = "menu-subtitle";
    /// Info card title.
    pub const CARD_TITLE: &str = "menu-card-title";
    /// Info card, no map.
    pub const CARD_NO_MAP: &str = "menu-card-no-map";
    /// Info card, map preview (variable: `path`).
    pub const CARD_MAP_PREVIEW: &str = "menu-card-map-preview";
    /// Info card, storage line.
    pub const CARD_STORAGE: &str = "menu-card-storage";
    /// Footer line.
    pub const FOOTER: &str = "menu-footer";
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
    menu::LOAD_LAST_GAME,
    menu::START_NEW_GAME,
    menu::SETTINGS,
    menu::CREDITS,
    menu::THEME_LIGHT,
    menu::THEME_DARK,
    menu::TITLE,
    menu::SUBTITLE,
    menu::CARD_TITLE,
    menu::CARD_NO_MAP,
    menu::CARD_MAP_PREVIEW,
    menu::CARD_STORAGE,
    menu::FOOTER,
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
