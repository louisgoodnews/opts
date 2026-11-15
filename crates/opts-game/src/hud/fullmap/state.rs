//! The full-size map's state: whether it is open, its view, what it drew, and the input hold.

use super::*;

/// What the last redraw showed: the view and the markers (position, heading, active).
pub(super) type DrawnState = (Viewport, Vec<(DVec2, f32, bool)>);

/// A request from outside the keyboard (the control protocol).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FullMapRequest {
    /// Open the map (a no-op while it is open or another dialog holds the input).
    Open,
    /// Close the map.
    Close,
}

/// What the full-size map shows and did last (read by `hud.fullmap` and the tests).
#[derive(Resource, Debug, Clone)]
pub struct FullMapState {
    /// The map is shown.
    pub open: bool,
    /// What part of the world the screen shows.
    pub viewport: Viewport,
    /// The centre follows the active vehicle (cleared by a drag, set again by Home).
    pub follow: bool,
    /// The span kept between openings.
    pub remembered_span_m: f64,
    /// Redraws since the session started.
    pub frames: u64,
    /// Time of the last redraw in microseconds.
    pub last_us: u64,
    /// The slowest redraw in microseconds.
    pub worst_us: u64,
    /// Road chords drawn by the last redraw.
    pub lines: usize,
    /// Markers drawn by the last redraw.
    pub markers: usize,
    /// Side of the texture in pixels.
    pub texture_px: u32,
    /// The logical size of the screen as last seen.
    pub screen: Vec2,
    /// A request from the protocol, handled by the next frame.
    pub request: Option<FullMapRequest>,
    /// Fit the whole map into the screen in the next frame (the protocol or a key).
    pub fit_request: bool,
    /// The protocol set the view: opening keeps it instead of centring on the vehicle.
    pub preset_view: bool,
    /// The overview generation the last redraw drew.
    pub(super) drawn_generation: u64,
    /// What the last redraw showed.
    pub(super) drawn: Option<DrawnState>,
    /// A change the user made (drag, wheel, Home): redraw this frame, not at the next tick.
    pub(super) immediate: bool,
    /// Seconds (real time) since the last redraw.
    pub(super) since: f32,
    /// The input stays held for a frame or two after closing, so the Esc press that closed the
    /// map does not also open the pause overlay.
    pub(super) grace: u8,
    /// Where the cursor was when the drag began or last moved.
    pub(super) drag_from: Option<Vec2>,
}

impl Default for FullMapState {
    fn default() -> Self {
        Self {
            open: false,
            viewport: Viewport::default(),
            follow: true,
            remembered_span_m: viewport::DEFAULT_SPAN_M,
            frames: 0,
            last_us: 0,
            worst_us: 0,
            lines: 0,
            markers: 0,
            texture_px: 0,
            screen: FALLBACK_SCREEN,
            request: None,
            fit_request: false,
            preset_view: false,
            drawn_generation: 0,
            drawn: None,
            immediate: false,
            since: 0.0,
            grace: 0,
            drag_from: None,
        }
    }
}

impl FullMapState {
    /// Whether the map owns the keyboard and the mouse: open, or just closed (see `grace`).
    #[must_use]
    pub fn holds_input(&self) -> bool {
        self.open || self.grace > 0
    }

    /// Asks for a redraw in the next frame (the view was changed from outside the mouse).
    pub fn touch(&mut self) {
        self.immediate = true;
    }

    pub(super) fn close(&mut self) {
        if self.open {
            self.remembered_span_m = self.viewport.span_m;
            self.open = false;
            self.grace = 2;
            self.drag_from = None;
        }
    }
}
