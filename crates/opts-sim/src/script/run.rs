//! running the blocks: init, frames, triggers, macros and the drains.

use super::*;

impl ScriptEngine {
    /// Runs every `{init}` block once.
    pub fn run_init(&mut self) {
        for i in 0..self.program.init.len() {
            let b = self.program.init[i];
            self.run_top(b);
        }
    }

    /// Advances `dt` seconds: updates `Timegap`/`GetTime` and runs the `{frame}` blocks
    /// (`{frame_ai}` instead when `ai` is set).
    pub fn run_frame(&mut self, dt: f32, ai: bool) {
        self.time += f64::from(dt);
        self.set_system("timegap", dt);
        // The time is kept in f64 and handed over wrapped at 65,536 s (about 18 h): an f32 of
        // the running total loses its milliseconds after a few hours, and scripts only compare
        // two readings.
        self.set_system("gettime", (self.time % GET_TIME_WRAP_S) as f32);
        let count = if ai {
            self.program.frame_ai.len()
        } else {
            self.program.frame.len()
        };
        for i in 0..count {
            let b = if ai {
                self.program.frame_ai[i]
            } else {
                self.program.frame[i]
            };
            self.run_top(b);
        }
    }

    /// Raises trigger `name` (key, hot-spot or script): runs its blocks; returns whether any exists.
    pub fn trigger(&mut self, name: &str) -> bool {
        let Some(id) = self.program.names.events.get(name) else {
            return false;
        };
        let blocks = self.program.triggers.get(&id).cloned().unwrap_or_default();
        for b in &blocks {
            self.run_top(*b);
        }
        !blocks.is_empty()
    }

    /// Whether any script block answers trigger `name`.
    #[must_use]
    pub fn has_trigger(&self, name: &str) -> bool {
        self.program.names.events.get(name).is_some_and(|id| {
            self.program
                .triggers
                .get(&id)
                .is_some_and(|b| !b.is_empty())
        })
    }

    /// Runs the macro `name` once on a fresh stack (diagnostics and tests).
    pub fn call_macro(&mut self, name: &str) -> bool {
        let Some(id) = self.program.names.macros.get(name) else {
            return false;
        };
        let Some(block) = self.program.macros.get(&id).copied() else {
            return false;
        };
        self.run_top(block);
        true
    }

    /// Takes the events raised since the last call.
    pub fn drain_events(&mut self) -> Vec<ScriptEvent> {
        std::mem::take(&mut self.events)
    }

    /// Takes the `$msg` console lines since the last call.
    pub fn drain_messages(&mut self) -> Vec<String> {
        std::mem::take(&mut self.messages)
    }

    /// Takes the run-time diagnostics (each distinct problem is reported once).
    pub fn drain_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Re-resolves constant and curve names to indices after the table changed.
    pub(super) fn refresh_consts(&mut self) {
        if !self.consts_dirty {
            return;
        }
        self.consts_dirty = false;
        let names = &self.program.names;
        self.resolved_consts = (0..names.consts.len() as u32)
            .map(|i| self.consts.scalar(names.consts.name(i)))
            .collect();
        self.resolved_curves = (0..names.curves.len() as u32)
            .map(|i| self.consts.curve(names.curves.name(i)).cloned())
            .collect();
        self.missing_seen.clear();
    }

    pub(super) fn run_top(&mut self, block: usize) {
        self.refresh_consts();
        let mut stacks = Stacks::default();
        self.run_block(block, 0, &mut stacks);
    }
}
