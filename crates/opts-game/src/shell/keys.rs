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
    /// Home text `home-welcome-back`.
    HOME_WELCOME_BACK = "home-welcome-back",
    /// Home text `home-welcome-first`.
    HOME_WELCOME_FIRST = "home-welcome-first",
    /// Home text `home-continue`.
    HOME_CONTINUE = "home-continue",
    /// Home text `home-continue-drive`.
    HOME_CONTINUE_DRIVE = "home-continue-drive",
    /// Home text `home-recent`.
    HOME_RECENT = "home-recent",
    /// Home text `home-quick-title`.
    HOME_QUICK_TITLE = "home-quick-title",
    /// Home text `home-quick-body`.
    HOME_QUICK_BODY = "home-quick-body",
    /// Home text `home-quick-action`.
    HOME_QUICK_ACTION = "home-quick-action",
    /// Home text `home-status-ready`.
    HOME_STATUS_READY = "home-status-ready",
    /// Home text `home-status-refreshing`.
    HOME_STATUS_REFRESHING = "home-status-refreshing",
    /// Home text `home-status-none`.
    HOME_STATUS_NONE = "home-status-none",
    /// Home text `home-banner-no-omsi-title`.
    HOME_BANNER_NO_OMSI_TITLE = "home-banner-no-omsi-title",
    /// Home text `home-banner-no-omsi-body`.
    HOME_BANNER_NO_OMSI_BODY = "home-banner-no-omsi-body",
    /// Home text `home-banner-choose-folder`.
    HOME_BANNER_CHOOSE_FOLDER = "home-banner-choose-folder",
    /// Home text `home-banner-empty-title`.
    HOME_BANNER_EMPTY_TITLE = "home-banner-empty-title",
    /// Home text `home-banner-empty-body`.
    HOME_BANNER_EMPTY_BODY = "home-banner-empty-body",
    /// Home text `home-reason-no-omsi`.
    HOME_REASON_NO_OMSI = "home-reason-no-omsi",
    /// Home text `home-reason-empty`.
    HOME_REASON_EMPTY = "home-reason-empty",
    /// Home text `home-reason-reading`.
    HOME_REASON_READING = "home-reason-reading",
    /// Home text `home-time-weather`.
    HOME_TIME_WEATHER = "home-time-weather",
    /// Home text `home-vehicle-livery`.
    HOME_VEHICLE_LIVERY = "home-vehicle-livery",
    /// Message `firstrun-found-title`.
    FIRSTRUN_FOUND_TITLE = "firstrun-found-title",
    /// Message `firstrun-found-body`.
    FIRSTRUN_FOUND_BODY = "firstrun-found-body",
    /// Message `firstrun-use`.
    FIRSTRUN_USE = "firstrun-use",
    /// Message `firstrun-other`.
    FIRSTRUN_OTHER = "firstrun-other",
    /// Message `firstrun-quit`.
    FIRSTRUN_QUIT = "firstrun-quit",
    /// Message `firstrun-language`: names the *other* language in its own language.
    FIRSTRUN_LANGUAGE = "firstrun-language",
    /// Message `firstrun-choose-title`.
    FIRSTRUN_CHOOSE_TITLE = "firstrun-choose-title",
    /// Message `firstrun-choose-body`.
    FIRSTRUN_CHOOSE_BODY = "firstrun-choose-body",
    /// Message `firstrun-choose-action`.
    FIRSTRUN_CHOOSE_ACTION = "firstrun-choose-action",
    /// Message `firstrun-wrong-folder`.
    FIRSTRUN_WRONG_FOLDER = "firstrun-wrong-folder",
    /// Message `firstrun-cancelled`.
    FIRSTRUN_CANCELLED = "firstrun-cancelled",
    /// Message `firstrun-error-title`.
    FIRSTRUN_ERROR_TITLE = "firstrun-error-title",
    /// Message `firstrun-save-failed-title`.
    FIRSTRUN_SAVE_FAILED_TITLE = "firstrun-save-failed-title",
    /// Message `firstrun-save-failed-body`.
    FIRSTRUN_SAVE_FAILED_BODY = "firstrun-save-failed-body",
    /// Message `firstrun-welcome-title`.
    FIRSTRUN_WELCOME_TITLE = "firstrun-welcome-title",
    /// Message `firstrun-welcome-body`.
    FIRSTRUN_WELCOME_BODY = "firstrun-welcome-body",
    /// Message `firstrun-pick`.
    FIRSTRUN_PICK = "firstrun-pick",
    /// Message `firstrun-quick`.
    FIRSTRUN_QUICK = "firstrun-quick",
    /// Message `home-stage-files`.
    HOME_STAGE_FILES = "home-stage-files",
    /// Message `home-stage-content`.
    HOME_STAGE_CONTENT = "home-stage-content",
    /// Message `home-stage-saving`.
    HOME_STAGE_SAVING = "home-stage-saving",
    /// Message `pause-title`.
    PAUSE_TITLE = "pause-title",
    /// Message `pause-resume`.
    PAUSE_RESUME = "pause-resume",
    /// Message `pause-vehicle`.
    PAUSE_VEHICLE = "pause-vehicle",
    /// Message `pause-settings`.
    PAUSE_SETTINGS = "pause-settings",
    /// Message `pause-controls`.
    PAUSE_CONTROLS = "pause-controls",
    /// Message `pause-report`.
    PAUSE_REPORT = "pause-report",
    /// Message `pause-leave`.
    PAUSE_LEAVE = "pause-leave",
    /// Message `leave-title`.
    LEAVE_TITLE = "leave-title",
    /// Message `leave-body`.
    LEAVE_BODY = "leave-body",
    /// Message `leave-home`.
    LEAVE_HOME = "leave-home",
    /// Message `leave-quit`.
    LEAVE_QUIT = "leave-quit",
    /// Message `summary-title`.
    SUMMARY_TITLE = "summary-title",
    /// Message `summary-duration`.
    SUMMARY_DURATION = "summary-duration",
    /// Message `summary-distance`.
    SUMMARY_DISTANCE = "summary-distance",
    /// Message `summary-max-speed`.
    SUMMARY_MAX_SPEED = "summary-max-speed",
    /// Message `summary-problems`.
    SUMMARY_PROBLEMS = "summary-problems",
    /// Message `summary-report`.
    SUMMARY_REPORT = "summary-report",
    /// Message `summary-again`.
    SUMMARY_AGAIN = "summary-again",
    /// Message `summary-change-vehicle`.
    SUMMARY_CHANGE_VEHICLE = "summary-change-vehicle",
    /// Message `summary-home`.
    SUMMARY_HOME = "summary-home",
    /// Message `summary-distance-m`.
    SUMMARY_DISTANCE_M = "summary-distance-m",
    /// Message `help-tab-controls`.
    HELP_TAB_CONTROLS = "help-tab-controls",
    /// Message `help-tab-guides`.
    HELP_TAB_GUIDES = "help-tab-guides",
    /// Message `help-tab-report`.
    HELP_TAB_REPORT = "help-tab-report",
    /// Message `help-tab-about`.
    HELP_TAB_ABOUT = "help-tab-about",
    /// Message `help-group-driving`.
    HELP_GROUP_DRIVING = "help-group-driving",
    /// Message `help-group-camera`.
    HELP_GROUP_CAMERA = "help-group-camera",
    /// Message `help-group-menus`.
    HELP_GROUP_MENUS = "help-group-menus",
    /// Message `help-key-throttle`.
    HELP_KEY_THROTTLE = "help-key-throttle",
    /// Message `help-key-steer`.
    HELP_KEY_STEER = "help-key-steer",
    /// Message `help-key-parking`.
    HELP_KEY_PARKING = "help-key-parking",
    /// Message `help-key-gears`.
    HELP_KEY_GEARS = "help-key-gears",
    /// Message `help-key-tram`.
    HELP_KEY_TRAM = "help-key-tram",
    /// Message `help-key-camera`.
    HELP_KEY_CAMERA = "help-key-camera",
    /// Message `help-key-camera-prev`.
    HELP_KEY_CAMERA_PREV = "help-key-camera-prev",
    /// Message `help-key-camera-reset`.
    HELP_KEY_CAMERA_RESET = "help-key-camera-reset",
    /// Message `help-key-viewpoint`.
    HELP_KEY_VIEWPOINT = "help-key-viewpoint",
    /// Message `help-key-pause-menu`.
    HELP_KEY_PAUSE_MENU = "help-key-pause-menu",
    /// Message `help-key-sim-pause`.
    HELP_KEY_SIM_PAUSE = "help-key-sim-pause",
    /// Message `help-key-vehicle-menu`.
    HELP_KEY_VEHICLE_MENU = "help-key-vehicle-menu",
    /// Message `help-key-help`.
    HELP_KEY_HELP = "help-key-help",
    /// Message `help-key-theme`.
    HELP_KEY_THEME = "help-key-theme",
    /// Message `help-key-dev`.
    HELP_KEY_DEV = "help-key-dev",
    /// Message `help-guides-body`.
    HELP_GUIDES_BODY = "help-guides-body",
    /// Message `help-report-body`.
    HELP_REPORT_BODY = "help-report-body",
    /// Message `help-report-action`.
    HELP_REPORT_ACTION = "help-report-action",
    /// Message `help-about-version`.
    HELP_ABOUT_VERSION = "help-about-version",
    /// Message `help-about-licence`.
    HELP_ABOUT_LICENCE = "help-about-licence",
    /// Message `help-about-clean`.
    HELP_ABOUT_CLEAN = "help-about-clean",
    /// Message `help-about-third`.
    HELP_ABOUT_THIRD = "help-about-third",
    /// Message `help-close`.
    HELP_CLOSE = "help-close",
    /// Message `help-overlay-title`.
    HELP_OVERLAY_TITLE = "help-overlay-title",
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
    /// Help: the quick-switch key of the session vehicles.
    HELP_KEY_SWITCH_VEHICLE = "help-key-switch-vehicle",
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
