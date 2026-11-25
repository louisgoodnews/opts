//! Rate-limited debug logs of the AI lamp variables: the session's dark state on every change, and
//! the lamp values of a few vehicles per window (so a test run shows what the vehicles are fed).

/// Seconds between two windows of lamp samples.
const WINDOW_S: f32 = 5.0;
/// Vehicles sampled per window.
const SAMPLES_PER_WINDOW: u32 = 3;

/// State of the lamp logs, kept by the system that feeds the lamp variables.
#[derive(Debug, Default)]
pub struct LampLog {
    dark: Option<bool>,
    window_end_s: f32,
    taken: u32,
}

impl LampLog {
    /// Logs `dark` once when it differs from the last value seen (the first value always logs).
    pub fn note_dark(&mut self, dark: bool) {
        if self.dark != Some(dark) {
            tracing::info!(dark, "session dark state changed (AI lamps)");
            self.dark = Some(dark);
        }
    }

    /// Whether the vehicle being fed now is one of the sampled ones: at most
    /// [`SAMPLES_PER_WINDOW`] per [`WINDOW_S`] seconds of `now`.
    pub fn take_sample(&mut self, now: f32) -> bool {
        if now >= self.window_end_s {
            self.window_end_s = now + WINDOW_S;
            self.taken = 0;
        }
        if self.taken < SAMPLES_PER_WINDOW {
            self.taken += 1;
            true
        } else {
            false
        }
    }
}

/// Logs the lamp values of one sampled vehicle at debug level.
pub fn log_lamps(agent: u64, vars: &opts_bevy::AiVariables) {
    tracing::debug!(
        agent,
        headlights = vars.get("lights_abbl_bulb_1"),
        tail = vars.get("lights_rueck"),
        brake = vars.get("lights_brems"),
        high_beam = vars.get("lights_fern"),
        blinker_l = vars.get("lights_blinker_l_bulb_1"),
        blinker_r = vars.get("lights_blinker_r_bulb_1"),
        blinkgeber = vars.get("lights_blinkgeber"),
        "AI lamps"
    );
}
