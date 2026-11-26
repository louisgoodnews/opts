//! Shell chrome keys: the rail, its footer, the quit and despawn dialogs, and session vehicle toasts.

use opts_shell_keys_help::keys;

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
    /// Toast: a destination could not be opened (variable `screen`).
    NAV_REFUSED = "shell-nav-refused",
    /// Despawn confirmation: title.
    DESPAWN_TITLE = "despawn-title",
    /// Despawn confirmation: the consequence (variable `name`).
    DESPAWN_BODY = "despawn-body",
    /// Despawn confirmation: the destructive button.
    DESPAWN_CONFIRM = "despawn-confirm",
    /// Toast: a session vehicle vanished (variable `name`).
    VEHICLE_LOST = "session-vehicle-lost",
    /// Toast: a vehicle could not be spawned (variables `name`, `error`).
    SPAWN_FAILED = "session-spawn-failed",
    /// Marker before the active vehicle in the vehicle list.
    SESSION_ACTIVE_MARK = "session-active-mark",
}
