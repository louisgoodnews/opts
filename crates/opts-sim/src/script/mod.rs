//! OMSI script engine: compiles `.osc` scripts to bytecode and runs them on a stack machine.
//!
//! Pure Rust and deterministic (the only randomness is a seeded generator); it knows nothing
//! about Bevy. A host feeds system values and constants, calls [`ScriptEngine::run_frame`] and
//! [`ScriptEngine::trigger`], reads variables and drains events. Design:
//! `docs/src/architecture/script-vm.md`; language: `docs/src/formats/text/vehicles/osc.md`.

mod ai_response;
pub mod callbacks;
mod compiler;
pub mod consist;
pub mod inputs;
mod inspect;
mod lexer;
mod load;
mod strings;
mod variables;
mod vehicle;
mod vm;

use std::collections::HashSet;

pub use ai_response::{measure_ai_response, AiResponse};
pub use compiler::{
    BinOp, Block, Compiler, Diagnostic, EventKind, Names, Op, Program, Severity, StrOp, UnOp,
};
pub use lexer::{tokenize, Token};
pub use load::{load_scenery_scripts, load_script_files, SceneryScriptSet, ScriptFiles};
pub use strings::Stacks;
pub use variables::{ConstTable, Curve, Interner, VariableStore};
pub use vehicle::load_vehicle_scripts;

/// Handler for `(M.V.name)` macros of vehicle-external systems: receives the name and both
/// operand stacks.
/// Handler of the `(M.V.name)` callbacks of the host (name in lower case, arguments on the stacks).
/// Returns `false` for a callback it does not implement; that is reported once.
pub type ExternalHandler = Box<dyn FnMut(&str, &mut Stacks) -> bool + Send>;

/// `GetTime` wraps after this many seconds (see [`ScriptEngine::run_frame`]).
pub const GET_TIME_WRAP_S: f64 = 65_536.0;

/// The names a script set reads and writes (`ScriptEngine::usage`), in lower case.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptUsage {
    /// Float variables read (`(L.L.x)`).
    pub reads: std::collections::BTreeSet<String>,
    /// Float variables written (`(S.L.x)`).
    pub writes: std::collections::BTreeSet<String>,
    /// String variables read.
    pub string_reads: std::collections::BTreeSet<String>,
    /// String variables written.
    pub string_writes: std::collections::BTreeSet<String>,
    /// System values read (`(L.S.x)`).
    pub system_reads: std::collections::BTreeSet<String>,
    /// Callbacks called (`(M.V.x)`).
    pub external: std::collections::BTreeSet<String>,
}

/// An event raised by a script with `(T.x.name)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEvent {
    /// Where it goes.
    pub kind: EventKind,
    /// Event name (lower case).
    pub name: String,
}

/// Float and string variables of an engine by name ([`ScriptEngine::export_variables`]).
pub type ExportedVariables = (Vec<(String, f32)>, Vec<(String, String)>);

/// One vehicle's scripts, variables and constants.
pub struct ScriptEngine {
    pub(crate) program: Program,
    pub(crate) vars: VariableStore,
    pub(crate) consts: ConstTable,
    pub(crate) sys: Vec<f32>,
    pub(crate) events: Vec<ScriptEvent>,
    pub(crate) messages: Vec<String>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) warned: HashSet<String>,
    pub(crate) rng: u32,
    pub(crate) external: Option<ExternalHandler>,
    pub(crate) resolved_consts: Vec<Option<f32>>,
    pub(crate) resolved_curves: Vec<Option<Curve>>,
    pub(crate) missing_seen: HashSet<(u8, u32)>,
    consts_dirty: bool,
    pub(super) declared: Option<HashSet<String>>,
    pub(crate) ops_executed: u64,
    pub(crate) callback_stats: callbacks::CallbackStats,
    time: f64,
}

impl ScriptEngine {
    /// Creates an engine for `program`; compile diagnostics stay with the caller.
    #[must_use]
    pub fn new(program: Program) -> Self {
        Self {
            program,
            vars: VariableStore::default(),
            consts: ConstTable::default(),
            sys: Vec::new(),
            events: Vec::new(),
            messages: Vec::new(),
            diagnostics: Vec::new(),
            warned: HashSet::new(),
            rng: 0x2545_F491,
            external: None,
            resolved_consts: Vec::new(),
            resolved_curves: Vec::new(),
            missing_seen: HashSet::new(),
            consts_dirty: true,
            declared: None,
            ops_executed: 0,
            callback_stats: callbacks::CallbackStats::default(),
            time: 0.0,
        }
    }

    /// Compiles `(file name, text)` sources and builds an engine; returns the compile diagnostics.
    #[must_use]
    pub fn from_sources(sources: &[(&str, &str)]) -> (Self, Vec<Diagnostic>) {
        let mut compiler = Compiler::new();
        for (file, text) in sources {
            compiler.add_source(file, text);
        }
        let (program, diagnostics) = compiler.finish();
        (Self::new(program), diagnostics)
    }

    /// Constants and curves (`(C.L.x)`, `(F.L.x)`), read only.
    #[must_use]
    pub fn consts(&self) -> &ConstTable {
        &self.consts
    }

    /// Constants and curves (`(C.L.x)`, `(F.L.x)`).
    pub fn consts_mut(&mut self) -> &mut ConstTable {
        self.consts_dirty = true;
        &mut self.consts
    }

    /// Declares the float (`strings == false`) or string variables of a `varnamelist`; their
    /// order fixes the variable indices (`varnamelist.md`).
    pub fn declare_variables(&mut self, names: &[String], strings: bool) {
        for n in names {
            if strings {
                self.program.names.strvars.intern(n);
            } else {
                self.program.names.vars.intern(n);
                self.declared
                    .get_or_insert_with(HashSet::new)
                    .insert(n.to_lowercase());
            }
        }
    }

    /// Applies a parsed constant file (later entries override earlier ones).
    pub fn add_constfile(&mut self, file: &opts_core::formats::constfile::ConstFile) {
        for (name, value) in &file.scalars {
            self.consts_mut().set_scalar(name, *value);
        }
        for c in &file.curves {
            self.consts_mut()
                .set_curve(&c.name, Curve::new(c.points.clone()));
        }
    }

    /// Installs the handler for `(M.V.x)` macros.
    pub fn set_external_handler(&mut self, handler: ExternalHandler) {
        self.external = Some(handler);
    }

    /// Seeds the `random` generator (replays use the same seed).
    pub fn seed_random(&mut self, seed: u32) {
        self.rng = seed.max(1);
    }

    /// Sets a system value read by `(L.S.name)`.
    pub fn set_system(&mut self, name: &str, value: f32) {
        let id = self.system_id(name);
        self.set_system_id(id, value);
    }

    /// Index of system value `name` for [`Self::set_system_id`] (hosts that set the same values
    /// every frame resolve the names once).
    pub fn system_id(&mut self, name: &str) -> u32 {
        self.program.names.sys.intern(name)
    }

    /// Sets the system value with index `id` (from [`Self::system_id`]).
    pub fn set_system_id(&mut self, id: u32, value: f32) {
        let id = id as usize;
        if self.sys.len() <= id {
            self.sys.resize(id + 1, 0.0);
        }
        self.sys[id] = value;
    }

    /// Index of float variable `name` for [`Self::get_id`] / [`Self::set_id`].
    pub fn variable_id(&mut self, name: &str) -> u32 {
        self.program.names.vars.intern(name)
    }

    /// Reads the float variable with index `id` (from [`Self::variable_id`]).
    #[must_use]
    pub fn get_id(&self, id: u32) -> f32 {
        self.vars.float(id)
    }

    /// Writes the float variable with index `id` (from [`Self::variable_id`]).
    pub fn set_id(&mut self, id: u32, value: f32) {
        self.vars.set_float(id, value);
    }

    /// A fresh engine running the same compiled program with the same constants and variable
    /// declarations, but its own (zeroed) variables, events and generator state. Hosts that
    /// place one script set many times compile it once and fork it per instance.
    #[must_use]
    pub fn fork(&self) -> Self {
        let mut e = Self::new(self.program.clone());
        e.consts = self.consts.clone();
        e.declared = self.declared.clone();
        e.rng = self.rng;
        e
    }

    /// The value of scalar constant `name` (a const file's `[const]`), `None` when there is none.
    #[must_use]
    pub fn const_scalar(&self, name: &str) -> Option<f32> {
        self.consts.scalar(name)
    }

    /// Reads float variable `name` (`0` if the scripts never touched it).
    #[must_use]
    pub fn get(&self, name: &str) -> f32 {
        self.program
            .names
            .vars
            .get(name)
            .map_or(0.0, |id| self.vars.float(id))
    }

    /// Writes float variable `name` (host inputs such as pedal positions).
    pub fn set(&mut self, name: &str, value: f32) {
        let id = self.program.names.vars.intern(name);
        self.vars.set_float(id, value);
    }

    /// Reads string variable `name`.
    #[must_use]
    pub fn get_string(&self, name: &str) -> &str {
        self.program
            .names
            .strvars
            .get(name)
            .map_or("", |id| self.vars.string(id))
    }

    /// Writes string variable `name`.
    pub fn set_string(&mut self, name: &str, value: &str) {
        let id = self.program.names.strvars.intern(name);
        self.vars.set_string(id, value.to_owned());
    }

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
    fn refresh_consts(&mut self) {
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

    fn run_top(&mut self, block: usize) {
        self.refresh_consts();
        let mut stacks = Stacks::default();
        self.run_block(block, 0, &mut stacks);
    }
}
