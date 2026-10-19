//! The viewport of the full-size map: what part of the world the screen shows, and the maths of
//! panning and zooming. Pure (no Bevy world), so the pixel guarantees of M-0108 are unit tests.
//!
//! The map is always north-up. `span_m` is the number of metres across the **shorter** side of the
//! screen, so the same zoom shows the same scale on a wide and on a narrow window. Screen
//! positions are in logical pixels from the top left, like the cursor position of a window.

use bevy::math::{DVec2, Vec2};

/// The closest zoom: 50 m across the shorter side of the screen.
pub const MIN_SPAN_M: f64 = 50.0;
/// The widest zoom: 20 km across the shorter side of the screen.
pub const MAX_SPAN_M: f64 = 20_000.0;
/// The span a map opens with.
pub const DEFAULT_SPAN_M: f64 = 1_000.0;

/// The largest side of the drawing texture in pixels (a bigger window is drawn at this size and
/// scaled up; the redraw stays within its budget).
pub const MAX_CANVAS_PX: u32 = 1_024;
/// The smallest side of the drawing texture in pixels.
pub const MIN_CANVAS_PX: u32 = 256;

/// What the screen shows: the world point in its middle and the zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// The world point (east, north, metres) at the middle of the screen.
    pub centre: DVec2,
    /// Metres across the shorter side of the screen, within `MIN_SPAN_M..=MAX_SPAN_M`.
    pub span_m: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            centre: DVec2::ZERO,
            span_m: DEFAULT_SPAN_M,
        }
    }
}

impl Viewport {
    /// A viewport on `centre`; the span is clamped to the documented range.
    #[must_use]
    pub fn new(centre: DVec2, span_m: f64) -> Self {
        Self {
            centre,
            span_m: clamp_span(span_m),
        }
    }

    /// Screen pixels per metre for a screen of `screen` logical pixels.
    #[must_use]
    pub fn px_per_m(&self, screen: Vec2) -> f64 {
        f64::from(screen.min_element().max(1.0)) / self.span_m
    }

    /// The world point under the screen position `p`.
    #[must_use]
    pub fn screen_to_world(&self, p: Vec2, screen: Vec2) -> DVec2 {
        let ppm = self.px_per_m(screen);
        let from_middle = DVec2::new(
            f64::from(p.x) - f64::from(screen.x) / 2.0,
            f64::from(p.y) - f64::from(screen.y) / 2.0,
        );
        // Screen y points down, north points up.
        self.centre + DVec2::new(from_middle.x / ppm, -from_middle.y / ppm)
    }

    /// The screen position of the world point `w`.
    #[must_use]
    pub fn world_to_screen(&self, w: DVec2, screen: Vec2) -> Vec2 {
        let ppm = self.px_per_m(screen);
        let d = w - self.centre;
        Vec2::new(
            (f64::from(screen.x) / 2.0 + d.x * ppm) as f32,
            (f64::from(screen.y) / 2.0 - d.y * ppm) as f32,
        )
    }

    /// Drags the map by `delta` pixels: the world follows the cursor, so the centre moves the
    /// other way.
    pub fn pan_pixels(&mut self, delta: Vec2, screen: Vec2) {
        let ppm = self.px_per_m(screen);
        self.centre.x -= f64::from(delta.x) / ppm;
        self.centre.y += f64::from(delta.y) / ppm;
    }

    /// Zooms by `factor` (above 1 zooms in) so that the world point under `cursor` stays under
    /// it. The span is clamped; at the limit nothing moves.
    pub fn zoom_at(&mut self, cursor: Vec2, screen: Vec2, factor: f64) {
        let anchor = self.screen_to_world(cursor, screen);
        let span = clamp_span(self.span_m / factor.max(1e-6));
        if (span - self.span_m).abs() < f64::EPSILON {
            return;
        }
        self.span_m = span;
        // The anchor is now somewhere else: shift the centre by the difference.
        let moved = self.screen_to_world(cursor, screen);
        self.centre += anchor - moved;
    }

    /// The half side (metres) of the square the drawing texture covers: the square that contains
    /// the screen, centred on [`Viewport::centre`].
    #[must_use]
    pub fn canvas_radius_m(&self, screen: Vec2) -> f64 {
        f64::from(screen.max_element().max(1.0)) / 2.0 / self.px_per_m(screen)
    }
}

/// `span_m` limited to the documented range.
#[must_use]
pub fn clamp_span(span_m: f64) -> f64 {
    span_m.clamp(MIN_SPAN_M, MAX_SPAN_M)
}

/// The side of the drawing texture for a screen: its longer side, within the texture limits.
#[must_use]
pub fn canvas_px(screen: Vec2) -> u32 {
    (screen.max_element().round() as u32).clamp(MIN_CANVAS_PX, MAX_CANVAS_PX)
}

/// The zoom factor of one wheel notch (a line of the wheel); a pixel-based wheel counts 1/40.
pub const WHEEL_LINE_FACTOR: f64 = 1.2;

/// The zoom factor of a wheel movement of `lines` lines (positive zooms in).
#[must_use]
pub fn wheel_factor(lines: f32) -> f64 {
    WHEEL_LINE_FACTOR.powf(f64::from(lines))
}

/// A round length (1, 2 or 5 times a power of ten metres) that is at most `max_m`, for the scale
/// bar, and its text part: metres below 1 km, kilometres from there.
#[must_use]
pub fn scale_bar_m(max_m: f64) -> f64 {
    if max_m <= 0.0 || !max_m.is_finite() {
        return 1.0;
    }
    let power = 10f64.powf(max_m.log10().floor());
    for m in [5.0, 2.0, 1.0] {
        if m * power <= max_m {
            return m * power;
        }
    }
    power
}
