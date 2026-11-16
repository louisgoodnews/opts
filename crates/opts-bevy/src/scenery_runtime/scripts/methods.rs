//! The engine of a placed scenery object: its construction, access to its variables and events.

use super::*;

impl SceneryScripts {
    /// Wraps a (forked) engine: writes the placed object's string variables (`names[i]` gets
    /// `values[i]`) and keeps the load diagnostics. `{init}` runs on the first tick.
    #[must_use]
    pub fn new(
        mut engine: ScriptEngine,
        names: &[String],
        values: &[String],
        load_diagnostics: Vec<Diagnostic>,
    ) -> Self {
        for (name, value) in names.iter().zip(values) {
            engine.set_string(name, value);
        }
        let ids = HostIds::resolve(&mut engine);
        let mut diagnostics = load_diagnostics;
        diagnostics.truncate(MAX_KEPT_DIAGNOSTICS);
        Self {
            engine: Mutex::new(engine),
            ids,
            initialised: false,
            diagnostics,
            ticks: 0,
            raised: Vec::new(),
        }
    }

    /// Runs `f` on the engine (to read or write variables); `None` if the lock is poisoned.
    pub fn with_engine<R>(&self, f: impl FnOnce(&mut ScriptEngine) -> R) -> Option<R> {
        self.engine.lock().ok().map(|mut e| f(&mut e))
    }

    /// The engine for exclusive access inside a system.
    pub fn engine_mut(&mut self) -> Option<&mut ScriptEngine> {
        self.engine.get_mut().ok()
    }

    /// Whether the script raised events that nobody has taken yet.
    #[must_use]
    pub fn has_raised(&self) -> bool {
        !self.raised.is_empty()
    }

    /// Takes the events the script raised since the last call (lower case names, oldest first).
    pub fn take_raised(&mut self) -> Vec<String> {
        std::mem::take(&mut self.raised)
    }

    /// Load-time and run-time diagnostics kept for this object.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
