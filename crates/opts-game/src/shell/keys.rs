//! Message keys of the shell (the tables are `l10n/shell-en.ftl` and `l10n/shell-de.ftl`).
//!
//! Key tables may hold string literals; everything else in the shell may not (see the test
//! `no_user_visible_literals_in_the_shell`).

macro_rules! keys {
    ($($(#[$m:meta])* $name:ident = $text:literal),* $(,)?) => {
        $($(#[$m])* pub const $name: &str = $text;)*
        /// Every key above; the completeness tests iterate it.
        pub const ALL: &[&str] = &[$($name),*];
    };
}

keys! {
    /// The brand line at the top of the rail.
    BRAND = "shell-brand",
    /// Rail: Home.
    NAV_HOME = "shell-nav-home",
    /// Rail: Drive.
    NAV_DRIVE = "shell-nav-drive",
    /// Rail: Library.
    NAV_LIBRARY = "shell-nav-library",
    /// Rail: Workshop.
    NAV_WORKSHOP = "shell-nav-workshop",
    /// Rail: Settings.
    NAV_SETTINGS = "shell-nav-settings",
    /// Rail: Help.
    NAV_HELP = "shell-nav-help",
    /// Rail footer: offers the light theme.
    THEME_TO_LIGHT = "shell-theme-to-light",
    /// Rail footer: offers the dark theme.
    THEME_TO_DARK = "shell-theme-to-dark",
    /// Rail footer: version (variable `version`).
    VERSION = "shell-version",
    /// Rail footer: quit.
    QUIT = "shell-quit",
    /// Quit dialog title.
    QUIT_TITLE = "shell-quit-title",
    /// Quit dialog body.
    QUIT_BODY = "shell-quit-body",
    /// Quit dialog: confirm.
    QUIT_CONFIRM = "shell-quit-confirm",
    /// Dialog: cancel.
    CANCEL = "shell-cancel",
    /// Home: heading.
    HOME_TITLE = "home-title",
    /// Home: New drive.
    HOME_NEW_DRIVE = "home-new-drive",
    /// Home: Load game.
    HOME_LOAD_GAME = "home-load-game",
    /// Help: heading.
    HELP_TITLE = "help-title",
    /// Help: about text.
    HELP_ABOUT = "help-about-text",
    /// Toast: a destination could not be opened (variable `screen`).
    NAV_REFUSED = "shell-nav-refused",
}

/// The key of the rail label of a registered screen id, if the shell knows one.
pub fn nav_title(id: &str) -> Option<&'static str> {
    Some(match id {
        "home" => NAV_HOME,
        "drive" => NAV_DRIVE,
        "library" => NAV_LIBRARY,
        "workshop" => NAV_WORKSHOP,
        "settings" => NAV_SETTINGS,
        "help" => NAV_HELP,
        _ => return None,
    })
}
