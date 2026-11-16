//! The key presses of a start procedure: one [`Step`] per press, and the constructors the recipes
//! use for them.

use super::ENGINE;

/// One key press of a start procedure.
#[derive(Debug, Clone, Copy)]
pub struct Step {
    /// The script trigger (`<trigger>_off` is raised on release).
    pub trigger: &'static str,
    /// The first degree at which the step is taken.
    pub from: f32,
    /// Seconds the key is held; for [`Self::until_engine`] the longest it is held.
    pub hold_s: f32,
    /// Hold until `engine_on` (the starter).
    pub until_engine: bool,
    /// Wait for the busbar voltage first.
    pub needs_busbar: bool,
    /// For a cockpit control turned with the mouse: the vertical mouse movement in pixels, spread
    /// over the first [`DRAG_S`] of the hold (`<trigger>_drag` each frame, like a real drag);
    /// 0 for a key press.
    pub drag_y: f32,
    /// Taken only while the engine is still off (a second try of a key that steps).
    pub only_if_off: bool,
}

/// Seconds over which a drag step moves the mouse.
pub const DRAG_S: f32 = 0.5;

pub(super) const fn tap(trigger: &'static str, from: f32) -> Step {
    Step {
        trigger,
        from,
        hold_s: 0.3,
        until_engine: false,
        needs_busbar: false,
        drag_y: 0.0,
        only_if_off: false,
    }
}

pub(super) const fn drag(trigger: &'static str, from: f32, drag_y: f32) -> Step {
    Step {
        trigger,
        from,
        hold_s: DRAG_S + 0.2,
        until_engine: false,
        needs_busbar: false,
        drag_y,
        only_if_off: false,
    }
}

/// The start key once more, held, if the first press left the engine off: on many vehicles the
/// start key itself steps (the first press switches the ignition on, the second runs the
/// starter: the TH Überlandbus S315/O550).
pub(super) const fn start_again(trigger: &'static str) -> Step {
    Step {
        only_if_off: true,
        ..start(trigger)
    }
}

pub(super) const fn start_drag(trigger: &'static str, drag_y: f32) -> Step {
    Step {
        drag_y,
        ..start(trigger)
    }
}

/// A button pressed once at the engine degree (the electrics on), not held for an engine that
/// fires.
pub(super) const fn press(trigger: &'static str) -> Step {
    Step {
        hold_s: 0.5,
        until_engine: false,
        ..start(trigger)
    }
}

pub(super) const fn start(trigger: &'static str) -> Step {
    Step {
        trigger,
        from: ENGINE,
        hold_s: 8.0,
        until_engine: true,
        needs_busbar: true,
        drag_y: 0.0,
        only_if_off: false,
    }
}

pub(super) const BATTERY: &str = "cp_batterietrennschalter_toggle";

pub(super) const START: &str = "kw_m_enginestart";
