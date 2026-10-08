//! Convenient prelude re-exporting core Material Design 3 types, tokens, and widgets.

pub use crate::{
    components::*,
    interaction::*,
    layout::*,
    plugin::OptsGuiPlugin,
    theme::{
        ColorRole, ColorScheme, ElevationLevel, MaterialTheme, MaterialThemeBuilder, RadiusLevel,
        SpacingScale, TypeScale, TypeStyle,
    },
    tokens::{
        BackgroundRole, BorderRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
    },
};
