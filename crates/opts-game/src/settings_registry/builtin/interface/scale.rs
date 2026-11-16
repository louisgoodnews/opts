//! UI scale, fonts, density and reduced motion: the look of the controls.

use super::*;

/// UI scale, fonts, density and reduced motion.
pub(super) fn scale_and_fonts() -> Vec<D> {
    vec![
        D::new(
            "ui_scale",
            K::Int {
                min: 75,
                max: 200,
                step: 25,
            },
            "100",
            C::Interface,
            40,
            "set-ui-scale",
            "set-ui-scale-desc",
        )
        .presets(&["100", "125", "150", "200"], " %")
        .keywords(&["size", "zoom", "text size", "accessibility"]),
        D::new(
            "ui_font",
            K::Text,
            "",
            C::Interface,
            42,
            "set-ui-font",
            "set-ui-font-desc",
        )
        .keywords(&["font", "typeface", "dyslexia", "accessibility", "readable"]),
        D::new(
            "ui_font_size",
            K::Int {
                min: 12,
                max: 24,
                step: 1,
            },
            "14",
            C::Interface,
            44,
            "set-ui-font-size",
            "set-ui-font-size-desc",
        )
        .presets(&["12", "14", "16", "18", "20", "24"], " px")
        .keywords(&[
            "font",
            "text size",
            "base size",
            "accessibility",
            "larger text",
        ]),
        D::new(
            "density",
            K::Enum(DENSITIES),
            "comfortable",
            C::Interface,
            50,
            "set-density",
            "set-density-desc",
        )
        .keywords(&["spacing", "compact"]),
        D::new(
            "reduced_motion",
            K::Bool,
            "false",
            C::Interface,
            60,
            "set-reduced-motion",
            "set-reduced-motion-desc",
        )
        .keywords(&["animation", "accessibility", "transitions"]),
    ]
}
