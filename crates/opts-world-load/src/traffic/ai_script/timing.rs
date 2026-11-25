//! Timing of the near-vehicle script step and of the traffic world step (M-0103 criterion 1).
//!
//! Each stage keeps a window of [`WINDOW`] frames and logs one summary line when the window is
//! full, so the log holds one line per window and not one per frame. Grep for the message
//! `near-script timing`; the field `stage` is `script` (the near-vehicle scripts, the whole
//! [`super::step_ai_scripts`] call, `count` = engaged near vehicles) or `world` (the traffic world
//! step in `move_traffic`, `count` = agents in the world). `ms_mean` and `ms_max` are the step
//! times in milliseconds over the window, `count_mean` and `count_max` the counts.

/// Frames per summary line.
pub const WINDOW: u32 = 120;

/// The summary of one full window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub frames: u32,
    pub ms_mean: f64,
    pub ms_max: f64,
    pub count_mean: f64,
    pub count_max: u32,
}

/// Running sums of the frames of the current window.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Window {
    frames: u32,
    ms_sum: f64,
    ms_max: f64,
    count_sum: u64,
    count_max: u32,
}

impl Window {
    /// Adds one frame (its step time `ms` and its count). Returns the summary when the window is
    /// full, and starts the next window.
    pub fn push(&mut self, ms: f64, count: u32) -> Option<Summary> {
        self.frames += 1;
        self.ms_sum += ms;
        self.ms_max = self.ms_max.max(ms);
        self.count_sum += u64::from(count);
        self.count_max = self.count_max.max(count);
        if self.frames < WINDOW {
            return None;
        }
        let frames = f64::from(self.frames);
        let summary = Summary {
            frames: self.frames,
            ms_mean: self.ms_sum / frames,
            ms_max: self.ms_max,
            count_mean: self.count_sum as f64 / frames,
            count_max: self.count_max,
        };
        *self = Self::default();
        Some(summary)
    }

    /// Adds one frame and logs the summary of `stage` when the window is full.
    pub fn record(&mut self, stage: &'static str, ms: f64, count: u32) {
        if let Some(s) = self.push(ms, count) {
            tracing::debug!(
                stage,
                frames = s.frames,
                ms_mean = s.ms_mean,
                ms_max = s.ms_max,
                count_mean = s.count_mean,
                count_max = s.count_max,
                "near-script timing"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Window, WINDOW};

    #[test]
    fn no_summary_before_the_window_is_full() {
        let mut w = Window::default();
        for _ in 0..WINDOW - 1 {
            assert!(w.push(1.0, 0).is_none());
        }
        assert!(w.push(1.0, 0).is_some());
    }

    #[test]
    fn mean_and_max_over_the_window() {
        let mut w = Window::default();
        let mut last = None;
        for i in 0..WINDOW {
            // Times 0.0, 0.5, 1.0, ... ; counts 0, 1, 2, ... (mod 7).
            last = w.push(f64::from(i) * 0.5, i % 7);
        }
        let s = last.expect("a full window");
        let n = f64::from(WINDOW);
        let ms_mean = 0.5 * (n - 1.0) / 2.0;
        assert_eq!(s.frames, WINDOW);
        assert!((s.ms_mean - ms_mean).abs() < 1e-9);
        assert!((s.ms_max - 0.5 * (n - 1.0)).abs() < 1e-9);
        assert_eq!(s.count_max, 6);
        let count_sum: u32 = (0..WINDOW).map(|i| i % 7).sum();
        assert!((s.count_mean - f64::from(count_sum) / n).abs() < 1e-9);
    }

    #[test]
    fn a_new_window_starts_empty() {
        let mut w = Window::default();
        for _ in 0..WINDOW {
            w.push(10.0, 3);
        }
        // The max of the first window must not carry into the second.
        for _ in 0..WINDOW - 1 {
            w.push(1.0, 0);
        }
        let s = w.push(1.0, 0).expect("second window");
        assert!((s.ms_max - 1.0).abs() < 1e-9);
        assert_eq!(s.count_max, 0);
    }
}
