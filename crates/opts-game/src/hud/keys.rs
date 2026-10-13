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
    /// Hints in a compact layout: the same keys, shorter.
    HINT_CONTROLS_COMPACT = "hud-hint-controls-compact",
    /// Mini map: scale and orientation line (`metres`, `mode`).
    MINIMAP_CAPTION = "hud-minimap-caption",
    /// Mini map: north points up.
    MINIMAP_NORTH_UP = "hud-minimap-north-up",
    /// Mini map: the heading points up.
    MINIMAP_HEADING_UP = "hud-minimap-heading-up",
    /// Widget name.
    WIDGET_MINIMAP = "hud-widget-minimap",
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
    /// Alert: the viewpoint was stored.
    ALERT_VIEWPOINT_SAVED = "hud-alert-viewpoint-saved",
    /// Alert: the viewpoint could not be written.
    ALERT_VIEWPOINT_FAILED = "hud-alert-viewpoint-failed",
    /// Viewpoint editor: title.
    VIEWPOINT_TITLE = "hud-viewpoint-title",
    /// Viewpoint editor: the vehicle's view being edited (`index`, `total`).
    VIEWPOINT_SUBTITLE = "hud-viewpoint-subtitle",
    /// Viewpoint editor: sideways row.
    VIEWPOINT_X = "hud-viewpoint-x",
    /// Viewpoint editor: height row.
    VIEWPOINT_Y = "hud-viewpoint-y",
    /// Viewpoint editor: forward row.
    VIEWPOINT_Z = "hud-viewpoint-z",
    /// Viewpoint editor: field of view row.
    VIEWPOINT_FOV = "hud-viewpoint-fov",
    /// Viewpoint editor: an offset in centimetres (`value`).
    VIEWPOINT_OFFSET = "hud-viewpoint-offset",
    /// Viewpoint editor: an angle in degrees (`value`).
    VIEWPOINT_ANGLE = "hud-viewpoint-angle",
    /// Viewpoint editor: the decrease button.
    VIEWPOINT_DECREASE = "hud-viewpoint-decrease",
    /// Viewpoint editor: the increase button.
    VIEWPOINT_INCREASE = "hud-viewpoint-increase",
    /// Viewpoint editor: reset button.
    VIEWPOINT_RESET = "hud-viewpoint-reset",
    /// Viewpoint editor: cancel button.
    VIEWPOINT_CANCEL = "hud-viewpoint-cancel",
    /// Viewpoint editor: save button.
    VIEWPOINT_SAVE = "hud-viewpoint-save",
    /// Viewpoint editor: keyboard hint.
    VIEWPOINT_HINT = "hud-viewpoint-hint",
    /// Viewpoint editor: marker for unsaved changes.
    VIEWPOINT_UNSAVED = "hud-viewpoint-unsaved",
}
