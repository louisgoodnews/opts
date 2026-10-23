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
    /// Caption of the rail direction.
    CAPTION_DIRECTION = "hud-caption-direction",
    /// Caption of the rail power lever.
    CAPTION_LEVER = "hud-caption-lever",
    /// Rail direction: forward.
    RAIL_FORWARD = "hud-rail-forward",
    /// Rail direction: backwards.
    RAIL_REVERSE = "hud-rail-reverse",
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
    /// Status bar: clock, weather and the followed timetable (`time`, `weather`, `line`, `stop`,
    /// `due`, `delay`).
    CLOCK_TIMETABLE = "hud-clock-timetable",
    /// Hints: the most used keys.
    HINT_CONTROLS = "hud-hint-controls",
    /// Hints of a rail vehicle (`up`, `down`, `zero`, `forward`, `reverse`, `help`).
    HINT_CONTROLS_RAIL = "hud-hint-controls-rail",
    /// Hints of a rail vehicle in a compact layout.
    HINT_CONTROLS_RAIL_COMPACT = "hud-hint-controls-rail-compact",
    /// Hints in a compact layout: the same keys, shorter.
    HINT_CONTROLS_COMPACT = "hud-hint-controls-compact",
    /// Mouse driving: how to leave it or free the mouse.
    MOUSE_DRIVING_ON = "hud-mouse-driving-on",
    /// Mouse driving: the mouse is free; how to drive with it again.
    MOUSE_DRIVING_FREE = "hud-mouse-driving-free",
    /// Mini map: scale and orientation line (`metres`, `mode`).
    MINIMAP_CAPTION = "hud-minimap-caption",
    /// Mini map: scale line while the radius follows the speed (`metres`, `mode`).
    MINIMAP_CAPTION_AUTO = "hud-minimap-caption-auto",
    /// Mini map: north points up.
    MINIMAP_NORTH_UP = "hud-minimap-north-up",
    /// Mini map: the heading points up.
    MINIMAP_HEADING_UP = "hud-minimap-heading-up",
    /// Door camera: the door's name (`number`, `kind`, `zone`, `side`).
    DOOR_TITLE = "hud-door-title",
    /// Door camera: an entry door.
    DOOR_ENTRY = "hud-door-entry",
    /// Door camera: an exit door.
    DOOR_EXIT = "hud-door-exit",
    /// Door camera: at the front.
    DOOR_FRONT = "hud-door-front",
    /// Door camera: in the middle.
    DOOR_MIDDLE = "hud-door-middle",
    /// Door camera: at the rear.
    DOOR_REAR = "hud-door-rear",
    /// Door camera: on the left.
    DOOR_LEFT = "hud-door-left",
    /// Door camera: on the right.
    DOOR_RIGHT = "hud-door-right",
    /// Full-size map: the title when the session has no map name.
    FULLMAP_TITLE = "hud-fullmap-title",
    /// Full-size map: the title while the roads of the whole map are still being read (`map`,
    /// `percent`).
    FULLMAP_READING = "hud-fullmap-reading",
    /// Full-size map: the keys.
    FULLMAP_HINT = "hud-fullmap-hint",
    /// Full-size map: the scale bar in metres (`metres`).
    FULLMAP_SCALE_M = "hud-fullmap-scale-m",
    /// Full-size map: the scale bar in kilometres (`km`).
    FULLMAP_SCALE_KM = "hud-fullmap-scale-km",
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
    /// Alert: points ahead of a rail vehicle, the way is straight on (`distance`).
    ALERT_SWITCH_STRAIGHT = "hud-alert-switch-straight",
    /// Alert: points ahead, the way is to the left (`distance`).
    ALERT_SWITCH_LEFT = "hud-alert-switch-left",
    /// Alert: points ahead, the way is to the right (`distance`).
    ALERT_SWITCH_RIGHT = "hud-alert-switch-right",
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
    /// Virtual mirrors: the area of the left mirror.
    VMIRROR_LEFT = "hud-vmirror-left",
    /// Virtual mirrors: the area of the upper right mirror.
    VMIRROR_RIGHT_UPPER = "hud-vmirror-right-upper",
    /// Virtual mirrors: the area of the lower right mirror (wide-angle).
    VMIRROR_RIGHT_LOWER = "hud-vmirror-right-lower",
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
    /// Viewpoint editor: the save scope is this vehicle.
    VIEWPOINT_SCOPE_VEHICLE = "hud-viewpoint-scope-vehicle",
    /// Viewpoint editor: the save scope is every vehicle of the model.
    VIEWPOINT_SCOPE_MODEL = "hud-viewpoint-scope-model",
}
