//! Message keys of the master-detail pattern, the *Plan a drive* wizard and the loading screen.
//!
//! The strings live in `wizard_en.ftl` and `wizard_de.ftl`, merged into the `en` and `de` tables
//! by [`Localization`](super::Localization). Key tables may hold string literals.

/// Keys of the master-detail pattern.
pub mod md {
    /// Label (frame name) of the search field.
    pub const SEARCH_LABEL: &str = "md-search-label";
    /// Placeholder of the search field when the screen names no own key (variable `count`).
    pub const SEARCH: &str = "md-search";
    /// Header of the recent entries.
    pub const SECTION_RECENT: &str = "md-section-recent";
    /// Header of the starred entries.
    pub const SECTION_FAVOURITES: &str = "md-section-favourites";
    /// Header of the full list.
    pub const SECTION_ALL: &str = "md-section-all";
    /// Nothing matches the search (variable `query`).
    pub const NO_RESULTS: &str = "md-no-results";
    /// Button that removes search text and chips.
    pub const CLEAR_FILTER: &str = "md-clear-filter";
    /// Tooltip of an empty star.
    pub const STAR_ADD: &str = "md-star-add";
    /// Tooltip of a filled star.
    pub const STAR_REMOVE: &str = "md-star-remove";
}

/// Keys of the loading screen.
pub mod load {
    /// `load-title`.
    pub const LOAD_TITLE: &str = "load-title";
    /// `load-tile-map`.
    pub const LOAD_TILE_MAP: &str = "load-tile-map";
    /// `load-tile-vehicle`.
    pub const LOAD_TILE_VEHICLE: &str = "load-tile-vehicle";
    /// `load-no-vehicle-sub`.
    pub const LOAD_NO_VEHICLE_SUB: &str = "load-no-vehicle-sub";
    /// `load-tile-when`.
    pub const LOAD_TILE_WHEN: &str = "load-tile-when";
    /// `load-stage-catalogue`.
    pub const LOAD_STAGE_CATALOGUE: &str = "load-stage-catalogue";
    /// `load-stage-resolve`.
    pub const LOAD_STAGE_RESOLVE: &str = "load-stage-resolve";
    /// `load-stage-map`.
    pub const LOAD_STAGE_MAP: &str = "load-stage-map";
    /// `load-stage-light`.
    pub const LOAD_STAGE_LIGHT: &str = "load-stage-light";
    /// `load-stage-vehicle`.
    pub const LOAD_STAGE_VEHICLE: &str = "load-stage-vehicle";
    /// `load-stage-start`.
    pub const LOAD_STAGE_START: &str = "load-stage-start";
    /// `load-stage-ready`.
    pub const LOAD_STAGE_READY: &str = "load-stage-ready";
    /// `load-stage-failed`.
    pub const LOAD_STAGE_FAILED: &str = "load-stage-failed";
    /// `load-cancel`.
    pub const LOAD_CANCEL: &str = "load-cancel";
    /// `load-cancel-title`.
    pub const LOAD_CANCEL_TITLE: &str = "load-cancel-title";
    /// `load-cancel-body`.
    pub const LOAD_CANCEL_BODY: &str = "load-cancel-body";
    /// `load-cancel-keep`.
    pub const LOAD_CANCEL_KEEP: &str = "load-cancel-keep";
    /// `load-cancel-yes`.
    pub const LOAD_CANCEL_YES: &str = "load-cancel-yes";
    /// `load-ready-problems`.
    pub const LOAD_READY_PROBLEMS: &str = "load-ready-problems";
    /// `load-failed`.
    pub const LOAD_FAILED: &str = "load-failed";
    /// `load-details`.
    pub const LOAD_DETAILS: &str = "load-details";
    /// `load-continue`.
    pub const LOAD_CONTINUE: &str = "load-continue";
    /// `load-retry`.
    pub const LOAD_RETRY: &str = "load-retry";
    /// `load-choose-another`.
    pub const LOAD_CHOOSE_ANOTHER: &str = "load-choose-another";
    /// `load-back`.
    pub const LOAD_BACK: &str = "load-back";
    /// `load-problem-map`.
    pub const LOAD_PROBLEM_MAP: &str = "load-problem-map";
    /// `load-problem-vehicle`.
    pub const LOAD_PROBLEM_VEHICLE: &str = "load-problem-vehicle";
    /// `load-problem-no-model`.
    pub const LOAD_PROBLEM_NO_MODEL: &str = "load-problem-no-model";
    /// `load-problem-tiles`.
    pub const LOAD_PROBLEM_TILES: &str = "load-problem-tiles";
    /// `load-problem-objects`.
    pub const LOAD_PROBLEM_OBJECTS: &str = "load-problem-objects";
    /// `load-problem-files`.
    pub const LOAD_PROBLEM_FILES: &str = "load-problem-files";
    /// `load-cause-map-unreadable`.
    pub const LOAD_CAUSE_MAP_UNREADABLE: &str = "load-cause-map-unreadable";
    /// `load-cause-no-tiles`.
    pub const LOAD_CAUSE_NO_TILES: &str = "load-cause-no-tiles";
    /// `load-cause-no-vehicle-file`.
    pub const LOAD_CAUSE_NO_VEHICLE_FILE: &str = "load-cause-no-vehicle-file";
    /// `load-cause-vehicle-unreadable`.
    pub const LOAD_CAUSE_VEHICLE_UNREADABLE: &str = "load-cause-vehicle-unreadable";
    /// `load-controls-title`.
    pub const LOAD_CONTROLS_TITLE: &str = "load-controls-title";
    /// `load-controls-accelerate`.
    pub const LOAD_CONTROLS_ACCELERATE: &str = "load-controls-accelerate";
    /// `load-controls-brake`.
    pub const LOAD_CONTROLS_BRAKE: &str = "load-controls-brake";
    /// `load-controls-steer`.
    pub const LOAD_CONTROLS_STEER: &str = "load-controls-steer";
    /// `load-controls-parking`.
    pub const LOAD_CONTROLS_PARKING: &str = "load-controls-parking";
    /// `load-controls-gears`.
    pub const LOAD_CONTROLS_GEARS: &str = "load-controls-gears";
    /// `load-controls-cameras`.
    pub const LOAD_CONTROLS_CAMERAS: &str = "load-controls-cameras";
    /// `load-controls-pause`.
    pub const LOAD_CONTROLS_PAUSE: &str = "load-controls-pause";
    /// `load-controls-menu`.
    pub const LOAD_CONTROLS_MENU: &str = "load-controls-menu";
    /// `load-minor-toast`.
    pub const LOAD_MINOR_TOAST: &str = "load-minor-toast";
    /// `load-minor-title`.
    pub const LOAD_MINOR_TITLE: &str = "load-minor-title";
    /// `load-close`.
    pub const LOAD_CLOSE: &str = "load-close";
}

/// Every key above and below; the completeness tests iterate it.
pub const ALL: &[&str] = &[
    md::SEARCH_LABEL,
    md::SEARCH,
    md::SECTION_RECENT,
    md::SECTION_FAVOURITES,
    md::SECTION_ALL,
    md::NO_RESULTS,
    md::CLEAR_FILTER,
    md::STAR_ADD,
    md::STAR_REMOVE,
    wiz::WIZ_OPTION_IDENT_VISIBLE,
    load::LOAD_TITLE,
    load::LOAD_MINOR_TOAST,
    load::LOAD_MINOR_TITLE,
    load::LOAD_CLOSE,
    load::LOAD_TILE_MAP,
    load::LOAD_TILE_VEHICLE,
    load::LOAD_NO_VEHICLE_SUB,
    load::LOAD_TILE_WHEN,
    load::LOAD_STAGE_CATALOGUE,
    load::LOAD_STAGE_RESOLVE,
    load::LOAD_STAGE_MAP,
    load::LOAD_STAGE_LIGHT,
    load::LOAD_STAGE_VEHICLE,
    load::LOAD_STAGE_START,
    load::LOAD_STAGE_READY,
    load::LOAD_STAGE_FAILED,
    load::LOAD_CANCEL,
    load::LOAD_CANCEL_TITLE,
    load::LOAD_CANCEL_BODY,
    load::LOAD_CANCEL_KEEP,
    load::LOAD_CANCEL_YES,
    load::LOAD_READY_PROBLEMS,
    load::LOAD_FAILED,
    load::LOAD_DETAILS,
    load::LOAD_CONTINUE,
    load::LOAD_RETRY,
    load::LOAD_CHOOSE_ANOTHER,
    load::LOAD_BACK,
    load::LOAD_PROBLEM_MAP,
    load::LOAD_PROBLEM_VEHICLE,
    load::LOAD_PROBLEM_NO_MODEL,
    load::LOAD_PROBLEM_TILES,
    load::LOAD_PROBLEM_OBJECTS,
    load::LOAD_PROBLEM_FILES,
    load::LOAD_CAUSE_MAP_UNREADABLE,
    load::LOAD_CAUSE_NO_TILES,
    load::LOAD_CAUSE_NO_VEHICLE_FILE,
    load::LOAD_CAUSE_VEHICLE_UNREADABLE,
    load::LOAD_CONTROLS_TITLE,
    load::LOAD_CONTROLS_ACCELERATE,
    load::LOAD_CONTROLS_BRAKE,
    load::LOAD_CONTROLS_STEER,
    load::LOAD_CONTROLS_PARKING,
    load::LOAD_CONTROLS_GEARS,
    load::LOAD_CONTROLS_CAMERAS,
    load::LOAD_CONTROLS_PAUSE,
    load::LOAD_CONTROLS_MENU,
    wiz::WIZ_OPTION_VIS_GRILL_INVISIBLE,
    wiz::WIZ_OPTION_WIMPEL_VISIBILITY,
    wiz::WIZ_OPTION_VIS_WIMPEL_INVISIBLE,
    wiz::WIZ_TITLE,
    wiz::WIZ_STEP_WHERE,
    wiz::WIZ_STEP_WHAT,
    wiz::WIZ_STEP_WHEN,
    wiz::WIZ_STEP_TRIP,
    wiz::WIZ_STEP_REVIEW,
    wiz::WIZ_TRIP_SEARCH,
    wiz::WIZ_TRIP_EMPTY,
    wiz::WIZ_TRIP_NONE,
    wiz::WIZ_TRIP_NONE_HINT,
    wiz::WIZ_TRIP_NONE_DETAIL,
    wiz::WIZ_TRIP_DEPARTS,
    wiz::WIZ_TRIP_STOPS,
    wiz::WIZ_TRIP_TOUR,
    wiz::WIZ_TRIP_START_HINT,
    wiz::WIZ_REVIEW_TRIP_FROM,
    wiz::WIZ_BACK,
    wiz::WIZ_NEXT,
    wiz::WIZ_SKIP,
    wiz::WIZ_DRIVE,
    wiz::WIZ_CHANGE,
    wiz::WIZ_OPEN_SETTINGS,
    wiz::WIZ_REASON_NO_MAP,
    wiz::WIZ_REASON_NO_START,
    wiz::WIZ_REASON_NO_VEHICLE,
    wiz::WIZ_SAVE_FAILED,
    wiz::WIZ_DATE,
    wiz::WIZ_WHERE_SEARCH,
    wiz::WIZ_WHERE_EMPTY,
    wiz::WIZ_WHERE_START,
    wiz::MAP_CONFIG_OPEN,
    wiz::WIZ_WHERE_DATE,
    wiz::WIZ_WHERE_NO_STARTS,
    wiz::WIZ_WHERE_DESCRIPTION,
    wiz::WIZ_WHERE_REMEMBERED_MISSING,
    wiz::WIZ_FACT_YEARS,
    wiz::WIZ_FACT_TILES,
    wiz::WIZ_FACT_STARTS,
    wiz::WIZ_FACT_DEPOT,
    wiz::WIZ_WHAT_SEARCH,
    wiz::WIZ_WHAT_EMPTY,
    wiz::WIZ_WHAT_NONE,
    wiz::WIZ_WHAT_NONE_SUB,
    wiz::WIZ_WHAT_FORMATION,
    wiz::WIZ_WHAT_FORMATION_NOTE,
    wiz::WIZ_FORMATION_TITLE,
    wiz::WIZ_FORMATION_HINT,
    wiz::WIZ_FORMATION_HEAD,
    wiz::WIZ_FORMATION_TURNED,
    wiz::WIZ_FORMATION_UP,
    wiz::WIZ_FORMATION_TURN,
    wiz::WIZ_FORMATION_REMOVE,
    wiz::WIZ_FORMATION_ADD,
    wiz::WIZ_FORMATION_CLEAR,
    wiz::WIZ_FORMATION_DONE,
    wiz::WIZ_WHAT_NONE_BODY,
    wiz::WIZ_NO_VEHICLE,
    wiz::WIZ_CLASS_ROAD,
    wiz::WIZ_CLASS_RAIL,
    wiz::WIZ_CLASS_AIRCRAFT,
    wiz::WIZ_FACT_MASS,
    wiz::WIZ_WHAT_LIVERY,
    wiz::WIZ_WHAT_LIVERY_DEFAULT,
    wiz::WIZ_WHAT_LIVERY_LOADING,
    wiz::WIZ_WHAT_LIVERY_NONE,
    wiz::WIZ_CONFIGS_TITLE,
    wiz::WIZ_CONFIGS_NAME,
    wiz::WIZ_CONFIGS_SAVE,
    wiz::WIZ_CONFIGS_EMPTY,
    wiz::WIZ_CONFIGS_FAVOURITE,
    wiz::WIZ_CONFIGS_UNFAVOURITE,
    wiz::WIZ_CONFIGS_DUPLICATE,
    wiz::WIZ_CONFIGS_DELETE,
    wiz::WIZ_CONFIGS_DELETE_CONFIRM,
    wiz::WIZ_CONFIGS_COPY_SUFFIX,
    wiz::WIZ_CONFIGS_SAVED,
    wiz::WIZ_CONFIGS_LOADED,
    wiz::WIZ_CONFIGS_LOADED_PARTLY,
    wiz::WIZ_CONFIGS_LOAD_FAILED,
    wiz::WIZ_CONFIGS_SAVE_FAILED,
    wiz::WIZ_CONFIGS_FILE_PROBLEM,
    wiz::WIZ_CONFIGS_VEHICLE_MISSING,
    wiz::WIZ_CONFIGS_REPAINT_MISSING,
    wiz::WIZ_CONFIGS_OPTION_LEFT_OUT,
    wiz::WIZ_WHAT_CONFIGURE,
    wiz::WIZ_WHAT_NO_OPTIONS,
    wiz::WIZ_WHAT_OPTIONS_CHANGED,
    wiz::WIZ_WHAT_REMEMBERED_MISSING,
    wiz::WIZ_OPTIONS_TITLE,
    wiz::WIZ_OPTIONS_RESET,
    wiz::WIZ_OPTIONS_DONE,
    wiz::WIZ_WHEN_QUICK,
    wiz::WIZ_WHEN_EXACT,
    wiz::WIZ_WHEN_WEATHER,
    wiz::WIZ_WHEN_NOW,
    wiz::WIZ_WHEN_MORNING,
    wiz::WIZ_WHEN_NOON,
    wiz::WIZ_WHEN_EVENING,
    wiz::WIZ_WHEN_NIGHT,
    wiz::WIZ_WHEN_DAY,
    wiz::WIZ_WHEN_MONTH,
    wiz::WIZ_WHEN_YEAR,
    wiz::WIZ_WHEN_HOUR,
    wiz::WIZ_WHEN_MINUTE,
    wiz::WIZ_STEP_LESS,
    wiz::WIZ_STEP_MORE,
    wiz::WIZ_WHEN_ERA_HINT,
    wiz::WIZ_WHEN_SOURCE_MAP,
    wiz::WIZ_WHEN_SOURCE_NOW,
    wiz::WIZ_MONTH_1,
    wiz::WIZ_MONTH_2,
    wiz::WIZ_MONTH_3,
    wiz::WIZ_MONTH_4,
    wiz::WIZ_MONTH_5,
    wiz::WIZ_MONTH_6,
    wiz::WIZ_MONTH_7,
    wiz::WIZ_MONTH_8,
    wiz::WIZ_MONTH_9,
    wiz::WIZ_MONTH_10,
    wiz::WIZ_MONTH_11,
    wiz::WIZ_MONTH_12,
    wiz::WIZ_WEATHER_CLEAR,
    wiz::WIZ_WEATHER_CLEAR_EFFECT,
    wiz::WIZ_WEATHER_OVERCAST,
    wiz::WIZ_WEATHER_OVERCAST_EFFECT,
    wiz::WIZ_WEATHER_RAIN,
    wiz::WIZ_WEATHER_RAIN_EFFECT,
    wiz::WIZ_WEATHER_SNOW,
    wiz::WIZ_WEATHER_SNOW_EFFECT,
    wiz::WIZ_WEATHER_FOG,
    wiz::WIZ_WEATHER_FOG_EFFECT,
    wiz::WIZ_WEATHER_FX_CLOUDS,
    wiz::WIZ_WEATHER_FX_OVERCAST,
    wiz::WIZ_WEATHER_FX_CLEAR,
    wiz::WIZ_WEATHER_FX_RAIN,
    wiz::WIZ_WEATHER_FX_SNOW,
    wiz::WIZ_WEATHER_FX_FOG,
    wiz::WIZ_WEATHER_FX_WIND,
    wiz::WIZ_WEATHER_FX_LINE,
    wiz::WIZ_WEATHER_LIVE,
    wiz::WIZ_REVIEW_TITLE,
];

/// Keys of the *Plan a drive* wizard.
pub mod wiz;
