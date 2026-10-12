//! Report modal state machine, shared data and the input-gating run condition.

use std::time::Instant;

use bevy::prelude::*;

/// Where the F1 report flow currently is.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReportModalState {
    /// No report in progress.
    #[default]
    Inactive,
    /// F1 was pressed; waiting for the screenshot (and its PNG encoding).
    Capturing,
    /// The modal is visible and waiting for the user.
    Open,
}

/// Working data of the current report.
#[derive(Resource, Debug)]
pub struct ReportModalData {
    /// The captured frame, shown in the modal preview.
    pub screenshot: Option<Handle<Image>>,
    /// PNG bytes of the capture, ready to be written.
    pub png: Option<Vec<u8>>,
    /// The description, kept in sync with the text area.
    pub text: String,
    /// Whether virtual time was running when F1 was pressed.
    pub was_running_before: bool,
    /// Whether this flow paused virtual time (so it must unpause it again).
    pub paused_by_report: bool,
    /// When the capture was requested (timeout detection).
    pub capture_started_at: Instant,
    /// Why the last attempt to save failed (already in the user's language); shown inline.
    pub error: Option<String>,
}

impl Default for ReportModalData {
    fn default() -> Self {
        Self {
            screenshot: None,
            png: None,
            text: String::new(),
            was_running_before: true,
            paused_by_report: false,
            capture_started_at: Instant::now(),
            error: None,
        }
    }
}

impl ReportModalData {
    /// Clears everything except the pause bookkeeping.
    pub fn reset_for_new_report(&mut self, was_running: bool) {
        self.screenshot = None;
        self.png = None;
        self.text.clear();
        self.was_running_before = was_running;
        self.capture_started_at = Instant::now();
        self.error = None;
    }
}

/// `true` while a report is being captured or edited; all gated input systems stand still.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ModalActive(pub bool);

/// `true` while the in-game vehicle menu (Left Alt) is open: the cursor is free and the driving,
/// camera and cockpit inputs stand still, like during the report modal.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MenuModalActive(pub bool);

/// Run condition for every input-driven system that must not react while the report modal or
/// the in-game menu is up. The modals' own systems and the F1 trigger do not use it. A missing
/// resource (no `ReportingPlugin`, e.g. in tests) counts as "not active".
#[must_use]
pub fn not_modal_active(
    modal: Option<Res<ModalActive>>,
    menu: Option<Res<MenuModalActive>>,
) -> bool {
    !modal.is_some_and(|m| m.0) && !menu.is_some_and(|m| m.0)
}

/// Best-effort facts about the running session, filled in by the application (`opts-game`)
/// so this crate does not depend on game state. `None` is written as `(none)`.
#[derive(Resource, Debug, Default, Clone)]
pub struct ReportContext {
    /// Application version (`CARGO_PKG_VERSION`).
    pub version: Option<String>,
    /// Short git commit hash the binary was built from.
    pub git: Option<String>,
    /// Loaded map name.
    pub map: Option<String>,
    /// Active camera mode, e.g. `"3 External"`.
    pub camera: Option<String>,
    /// Player vehicle.
    pub vehicle: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_active_condition_follows_the_resource() {
        let mut world = World::new();
        world.insert_resource(ModalActive(false));
        let mut sys = IntoSystem::into_system(not_modal_active);
        sys.initialize(&mut world);
        assert!(sys.run((), &mut world).unwrap());
        world.insert_resource(ModalActive(true));
        assert!(!sys.run((), &mut world).unwrap());
    }
}
