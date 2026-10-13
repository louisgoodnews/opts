//! Message keys of the HUD (tables: `l10n/hud-en.ftl`, `l10n/hud-de.ftl`). Key tables may hold
//! string literals; the source test of `tests/shell_sources.rs` forbids them everywhere else.

macro_rules! keys {
    ($($(#[$m:meta])* $name:ident = $text:literal),* $(,)?) => {
        $($(#[$m])* pub const $name: &str = $text;)*
        /// Every key above; the completeness tests iterate it.
        pub const ALL: &[&str] = &[$($name),*];
    };
}

keys! {
    /// Unit caption, kilometres per hour.
    UNIT_KMH = "hud-unit-kmh",
    /// Unit caption, miles per hour.
    UNIT_MPH = "hud-unit-mph",
    /// Caption of the gear readout.
    CAPTION_GEAR = "hud-caption-gear",
    /// Caption of the engine speed readout.
    CAPTION_RPM = "hud-caption-rpm",
    /// Caption of the parking brake lamp.
    CAPTION_PARK = "hud-caption-park",
    /// Parking brake on.
    BRAKE_ON = "hud-brake-on",
    /// Parking brake off.
    BRAKE_OFF = "hud-brake-off",
    /// Status bar: map and entry point (`map`, `entry`).
    SESSION = "hud-session",
    /// Status bar: the map only, when no place label applies (`map`).
    SESSION_MAP = "hud-session-map",
    /// Status bar: clock and weather (`time`, `weather`).
    CLOCK = "hud-clock",
    /// Hints: the most used keys.
    HINT_CONTROLS = "hud-hint-controls",
    /// The pause button of the HUD.
    PAUSE_BUTTON = "hud-pause-button",
    /// Camera label: the human character.
    CAMERA_EGO = "hud-camera-ego",
    /// Camera label: the driver's view.
    CAMERA_COCKPIT = "hud-camera-cockpit",
    /// Camera label: the external view.
    CAMERA_EXTERNAL = "hud-camera-external",
    /// Camera label: a passenger view.
    CAMERA_PASSENGER = "hud-camera-passenger",
    /// Camera label: the free camera.
    CAMERA_FREE = "hud-camera-free",
    /// Camera label with a sub camera (`name`, `index`, `total`).
    CAMERA_SUB = "hud-camera-sub",
    /// Alert word: information.
    ALERT_INFO = "hud-alert-info",
    /// Alert word: warning.
    ALERT_WARNING = "hud-alert-warning",
    /// Alert word: critical.
    ALERT_CRITICAL = "hud-alert-critical",
    /// Alert line: word and text (`word`, `text`).
    ALERT_LINE = "hud-alert-line",
    /// Alert: the parking brake is on while the vehicle moves.
    ALERT_BRAKE_MOVING = "hud-alert-brake-moving",
    /// Alert: a test alert raised through the control protocol.
    ALERT_TEST = "hud-alert-test",
    /// Widget names.
    WIDGET_SPEED = "hud-widget-speed",
    /// Widget name.
    WIDGET_GEAR = "hud-widget-gear",
    /// Widget name.
    WIDGET_RPM = "hud-widget-rpm",
    /// Widget name.
    WIDGET_PARKING_BRAKE = "hud-widget-parking-brake",
    /// Widget name.
    WIDGET_SESSION = "hud-widget-session",
    /// Widget name.
    WIDGET_CLOCK = "hud-widget-clock",
    /// Widget name.
    WIDGET_CONTROLS_HINT = "hud-widget-controls-hint",
    /// Widget name.
    WIDGET_ALERTS = "hud-widget-alerts",
    /// Widget name.
    WIDGET_CAMERA_LABEL = "hud-widget-camera-label",
    /// Details toast: the button label.
    DETAILS_ACTION = "hud-details-action",
    /// Details dialog: title.
    DETAILS_TITLE = "hud-details-title",
    /// Details dialog: body (`details`).
    DETAILS_BODY = "hud-details-body",
    /// Details dialog: copy.
    DETAILS_COPY = "hud-details-copy",
    /// Details dialog: report.
    DETAILS_REPORT = "hud-details-report",
    /// Details dialog: close.
    DETAILS_CLOSE = "hud-details-close",
    /// Toast after Copy.
    DETAILS_COPIED = "hud-details-copied",
}
