//! Named Material Icons codepoints and helpers for spawning icon text.
//!
//! Icons are single private-use characters rendered with the [`ICON_FONT`].
//! Use these instead of emoji: Bevy 0.14 cannot draw colour emoji and has no font fallback.

use bevy::prelude::*;

use crate::fonts::ICON_FONT;

macro_rules! icons {
    ($($name:ident = $cp:literal),* $(,)?) => {
        $(
            #[doc = concat!("Material icon `", stringify!($name), "` (U+", stringify!($cp), ").")]
            pub const $name: &str = $cp;
        )*
        /// Every named icon as `(name, glyph)`, used by the coverage test.
        pub const ALL: &[(&str, &str)] = &[$((stringify!($name), $name)),*];
    };
}

icons! {
    SEARCH = "\u{e8b6}",
    CLOSE = "\u{e5cd}",
    CHECK = "\u{e5ca}",
    INFO = "\u{e88e}",
    REFRESH = "\u{e5d5}",
    LOCATION_ON = "\u{e0c8}",
    DIRECTIONS_BUS = "\u{e530}",
    THUNDERSTORM = "\u{ebdb}",
    FOGGY = "\u{e818}",
    SUNNY = "\u{e430}",
    ARROW_DROP_DOWN = "\u{e5c5}",
    PLAY_ARROW = "\u{e037}",
    DOT = "\u{e061}",
}

/// Font and colour of an icon glyph rendered with the Material Icons font.
#[must_use]
pub fn icon_style(size: f32, color: Color) -> (TextFont, TextColor) {
    (
        TextFont {
            font: FontSource::Handle(ICON_FONT),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

/// A text bundle showing `icon` (one of the constants in this module).
#[must_use]
pub fn icon_text(icon: &str, size: f32, color: Color) -> impl Bundle {
    (Text::new(icon), icon_style(size, color))
}
