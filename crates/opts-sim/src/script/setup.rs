//! engine construction, constants, external handler and seeding.

use super::*;

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
}
