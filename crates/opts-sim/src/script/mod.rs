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
pub use load::{load_scenery_scripts, load_script_files, FileRead, SceneryScriptSet, ScriptFiles};
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

mod access;
mod run;
mod setup;
