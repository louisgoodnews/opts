//! The **instrument token set**: colours and sizes for HUD instruments (speed, gear, warning
//! lamps), separate from the application theme but never hard-coded in widgets.
//!
//! Two variants follow the time of day of the drive ([`InstrumentVariant::Day`] and
//! [`InstrumentVariant::Night`]); the night variant is dimmer and warmer so it never outshines a
//! night scene. The translucent panel is the one legitimate use of glass; the palette guarantees
//! readable text over **both a white and a black backdrop** (checked in the tests).

use bevy::color::Color;
use bevy::prelude::Resource;

/// Which instrument palette is active.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InstrumentVariant {
    /// Daylight: bright text on a dark translucent panel.
    #[default]
    Day,
    /// Night: dimmer, warmer, lower brightness.
    Night,
}

/// The colours of one instrument variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstrumentPalette {
    /// Translucent panel background (alpha < 1).
    pub panel: Color,
    /// Panel outline.
    pub panel_border: Color,
    /// Primary readout text (speed, gear).
    pub text: Color,
    /// Secondary text (units, captions).
    pub text_dim: Color,
    /// Normal state of a lamp or bar.
    pub ok: Color,
    /// Warning lamp.
    pub warning: Color,
    /// Alarm lamp (parking brake, door open, fault).
    pub alarm: Color,
    /// Needles and active bar segments.
    pub needle: Color,
    /// Inactive bar segments and gauge tracks.
    pub track: Color,
}

/// Day and night palettes plus the sizes shared by both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstrumentTokens {
    /// The daylight palette.
    pub day: InstrumentPalette,
    /// The night palette.
    pub night: InstrumentPalette,
    /// Size of the speed readout (the `NumericLarge` style).
    pub speed_size: f32,
    /// Size of the gear readout (the `NumericMedium` style).
    pub gear_size: f32,
    /// Corner radius of instrument panels.
    pub panel_radius: f32,
    /// Panel padding.
    pub panel_padding: f32,
}

impl InstrumentTokens {
    /// The palette of a variant.
    pub const fn palette(&self, variant: InstrumentVariant) -> &InstrumentPalette {
        match variant {
            InstrumentVariant::Day => &self.day,
            InstrumentVariant::Night => &self.night,
        }
    }
}

impl Default for InstrumentTokens {
    fn default() -> Self {
        Self {
            day: InstrumentPalette {
                panel: Color::srgba(0.05, 0.06, 0.08, 0.78),
                panel_border: Color::srgba(1.0, 1.0, 1.0, 0.35),
                text: Color::srgb(0.97, 0.97, 0.98),
                text_dim: Color::srgb(0.80, 0.82, 0.86),
                ok: Color::srgb(0.45, 0.88, 0.55),
                warning: Color::srgb(1.0, 0.78, 0.25),
                alarm: Color::srgb(1.0, 0.45, 0.40),
                needle: Color::srgb(0.62, 0.79, 1.0),
                track: Color::srgb(0.38, 0.40, 0.46),
            },
            night: InstrumentPalette {
                panel: Color::srgba(0.02, 0.02, 0.03, 0.82),
                panel_border: Color::srgba(1.0, 0.9, 0.8, 0.28),
                text: Color::srgb(0.86, 0.82, 0.76),
                text_dim: Color::srgb(0.68, 0.65, 0.61),
                ok: Color::srgb(0.38, 0.72, 0.46),
                warning: Color::srgb(0.90, 0.66, 0.20),
                alarm: Color::srgb(0.92, 0.38, 0.34),
                needle: Color::srgb(0.70, 0.62, 0.50),
                track: Color::srgb(0.30, 0.29, 0.28),
            },
            speed_size: 48.0,
            gear_size: 32.0,
            panel_radius: 12.0,
            panel_padding: 12.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast::{composite_over, contrast_ratio};

    fn readable_over(palette: &InstrumentPalette, backdrop: Color) -> Vec<(&'static str, f32)> {
        let panel = composite_over(palette.panel, backdrop);
        [
            ("text", palette.text, 4.5),
            ("text_dim", palette.text_dim, 4.5),
            ("ok", palette.ok, 3.0),
            ("warning", palette.warning, 3.0),
            ("alarm", palette.alarm, 3.0),
            ("needle", palette.needle, 3.0),
        ]
        .into_iter()
        .map(|(n, c, min)| (n, contrast_ratio(c, panel) / min))
        .collect()
    }

    #[test]
    fn every_instrument_colour_is_readable_over_a_bright_and_a_dark_scene() {
        let tokens = InstrumentTokens::default();
        for variant in [InstrumentVariant::Day, InstrumentVariant::Night] {
            for backdrop in [Color::WHITE, Color::BLACK, Color::srgb(0.5, 0.7, 1.0)] {
                for (name, ratio_over_min) in readable_over(tokens.palette(variant), backdrop) {
                    assert!(
                        ratio_over_min >= 1.0,
                        "{variant:?} {name} over {backdrop:?}: {ratio_over_min:.2}x of the minimum"
                    );
                }
            }
        }
    }

    #[test]
    fn night_is_dimmer_than_day() {
        let t = InstrumentTokens::default();
        let lum = |c: Color| crate::theme::contrast::relative_luminance(c);
        assert!(lum(t.night.text) < lum(t.day.text));
    }
}
