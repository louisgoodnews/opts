//! Mini map tests: the drawing, the contrast of its colours, the zoom steps and the budget.

use bevy::prelude::*;
use opts_gui::theme::contrast::contrast_ratio;
use opts_gui::theme::{InstrumentPalette, InstrumentTokens};
use opts_traffic::PathGeometry;

use super::raster::{render, Canvas, MapColors, MapLine, MapMarker, MapView};
use super::source::{push_chords, FixtureRoads, MapRoad, MapSource};
use super::*;

fn day() -> InstrumentPalette {
    InstrumentTokens::default().day
}

fn night() -> InstrumentPalette {
    InstrumentTokens::default().night
}

fn colors() -> MapColors {
    colors_of(&day())
}

const NORTH_UP: MapView = MapView {
    radius_m: 100.0,
    up_heading: 0.0,
};

fn centre_of(canvas: &Canvas) -> [u8; 4] {
    canvas.get(canvas.size / 2, canvas.size / 2)
}

fn line(a: (f32, f32), b: (f32, f32)) -> MapLine {
    line_w(a, b, 6.0)
}

fn line_w(a: (f32, f32), b: (f32, f32), width_m: f32) -> MapLine {
    MapLine {
        a: Vec2::new(a.0, a.1),
        b: Vec2::new(b.0, b.1),
        width_m,
    }
}

mod budget;
mod config;
mod draw;
mod palette;
mod paths;

// ---- speed zoom ------------------------------------------------------------------------------

#[path = "tests/speed_zoom/tests.rs"]
mod speed_zoom;
