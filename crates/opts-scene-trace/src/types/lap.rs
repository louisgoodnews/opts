//! Stage timing of the trace: a [`Lap`] measures only while the trace is on.

use std::time::Instant;

use super::super::enabled;

/// Measures a stage when the trace is on; a no-op `Lap` otherwise.
pub struct Lap(Option<Instant>);

impl Lap {
    /// Starts a lap if the trace is on.
    #[must_use]
    pub fn start() -> Self {
        Self(enabled().then(Instant::now))
    }

    /// Milliseconds since the last call (or start); 0 when off.
    pub fn split(&mut self) -> f32 {
        match &mut self.0 {
            Some(t) => {
                let now = Instant::now();
                let ms = now.duration_since(*t).as_secs_f32() * 1000.0;
                *t = now;
                ms
            }
            None => 0.0,
        }
    }

    /// Whether the lap measures.
    #[must_use]
    pub fn active(&self) -> bool {
        self.0.is_some()
    }
}
