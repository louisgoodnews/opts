//! Material Icons code points used by the patterns and the wizard (the same font as
//! [`crate::icons`]; these are the glyphs that table does not name yet).

macro_rules! glyphs {
    ($($name:ident = $cp:literal),* $(,)?) => {
        $(
            #[doc = concat!("Material icon `", stringify!($name), "`.")]
            pub const $name: &str = $cp;
        )*
    };
}

glyphs! {
    STAR = "\u{e838}",
    STAR_BORDER = "\u{e83a}",
    MAP = "\u{e55b}",
    TRAIN = "\u{e570}",
    FLIGHT = "\u{e539}",
    BUS = "\u{e530}",
    SCHEDULE = "\u{e8b5}",
    ARROW_BACK = "\u{e5c4}",
    ARROW_FORWARD = "\u{e5c8}",
    EDIT = "\u{e3c9}",
    SETTINGS = "\u{e8b8}",
    SUNNY = "\u{e430}",
    CLOUD = "\u{e2bd}",
    WATER_DROP = "\u{e798}",
    SNOW = "\u{eb3b}",
    NIGHT = "\u{ea46}",
    STORM = "\u{ebdb}",
    FOG = "\u{e818}",
    ADD = "\u{e145}",
    REMOVE = "\u{e15b}",
    TUNE = "\u{e429}",
    IMAGE = "\u{e3f4}",
    PLACE = "\u{e55f}",
    CHECK = "\u{e5ca}",
    CHECK_CIRCLE = "\u{e86c}",
    WARNING = "\u{e002}",
    ERROR = "\u{e000}",
    COPY = "\u{e14d}",
    EXPAND_MORE = "\u{e5cf}",
    EXPAND_LESS = "\u{e5ce}",
    PUBLIC = "\u{e80b}",
}
