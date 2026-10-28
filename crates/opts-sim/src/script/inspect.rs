//! Inspection, disassembly, variable export/import, and usage analysis for [`ScriptEngine`].

use super::*;

impl ScriptEngine {
    /// Float variables the scripts use although no varlist declared them (empty until
    /// [`Self::declare_variables`] was called). OMSI reads such names as 0.
    #[must_use]
    pub fn undeclared_variables(&self) -> Vec<String> {
        let Some(declared) = &self.declared else {
            return Vec::new();
        };
        let names = &self.program.names.vars;
        (0..names.len() as u32)
            .map(|i| names.name(i).to_owned())
            .filter(|n| !declared.contains(n))
            .collect()
    }

    /// Whether any block of the scripts stores to float variable `name` (`(S.L.name)`): the
    /// scripts *produce* the variable (e.g. `M_Wheel`), so the host can rely on it.
    #[must_use]
    pub fn writes(&self, name: &str) -> bool {
        let Some(id) = self.program.names.vars.get(name) else {
            return false;
        };
        self.program.blocks.iter().any(|b| {
            b.ops
                .iter()
                .any(|op| matches!(op, Op::Store(i) if *i == id))
        })
    }

    /// The state of the scripts as data: every float and string variable that is not zero or
    /// empty, by name, sorted. Together with the compiled program this is all a vehicle's scripts
    /// remember between frames (a session save stores it, see
    /// `docs/src/architecture/session-save.md`).
    #[must_use]
    pub fn export_variables(&self) -> ExportedVariables {
        let names = &self.program.names;
        let mut floats: Vec<(String, f32)> = self
            .vars
            .nonzero_floats()
            .map(|(i, v)| (names.vars.name(i).to_owned(), v))
            .collect();
        let mut strings: Vec<(String, String)> = self
            .vars
            .nonempty_strings()
            .map(|(i, v)| (names.strvars.name(i).to_owned(), v.to_owned()))
            .collect();
        floats.sort_by(|a, b| a.0.cmp(&b.0));
        strings.sort_by(|a, b| a.0.cmp(&b.0));
        (floats, strings)
    }

    /// Writes variables captured by [`Self::export_variables`] back; names this program does not
    /// have are created (hosts read them by name), so nothing is lost.
    pub fn import_variables(&mut self, floats: &[(String, f32)], strings: &[(String, String)]) {
        for (name, value) in floats {
            self.set(name, *value);
        }
        for (name, value) in strings {
            self.set_string(name, value);
        }
    }

    /// All float variables the scripts know, with their current values (diagnostics).
    #[must_use]
    pub fn variables(&self) -> Vec<(String, f32)> {
        let names = &self.program.names.vars;
        (0..names.len() as u32)
            .map(|i| (names.name(i).to_owned(), self.vars.float(i)))
            .collect()
    }

    /// All string variables the scripts know, with their current values (diagnostics).
    #[must_use]
    pub fn string_variables(&self) -> Vec<(String, String)> {
        let names = &self.program.names.strvars;
        (0..names.len() as u32)
            .map(|i| (names.name(i).to_owned(), self.vars.string(i).to_owned()))
            .collect()
    }

    /// Readable listing of the blocks whose name contains `name` (diagnostics).
    #[must_use]
    pub fn disassemble(&self, name: &str) -> Vec<String> {
        use compiler::Op;
        let n = &self.program.names;
        let mut out = Vec::new();
        for b in self.program.blocks.iter().filter(|b| b.name.contains(name)) {
            out.push(format!("== {} ({})", b.name, b.file));
            for (i, op) in b.ops.iter().enumerate() {
                let text = match *op {
                    Op::Load(v) => format!("L {}", n.vars.name(v)),
                    Op::Store(v) => format!("S {}", n.vars.name(v)),
                    Op::LoadStr(v) => format!("L$ {}", n.strvars.name(v)),
                    Op::StoreStr(v) => format!("S$ {}", n.strvars.name(v)),
                    Op::Const(v) => format!("C {}", n.consts.name(v)),
                    Op::Call(v) => format!("M {}", n.macros.name(v)),
                    other => format!("{other:?}"),
                };
                out.push(format!("{i:>4} {text}"));
            }
        }
        out
    }

    /// Instructions executed so far (profiling).
    #[must_use]
    pub fn ops_executed(&self) -> u64 {
        self.ops_executed
    }

    /// How often each callback was answered by the host, by the stub or not at all (unknown
    /// name) since the engine was built.
    #[must_use]
    pub fn callback_stats(&self) -> &callbacks::CallbackStats {
        &self.callback_stats
    }

    /// The variables the compiled scripts read and write, by name: what the integration report
    /// compares with the input table. `external` lists the callbacks they call.
    #[must_use]
    pub fn usage(&self) -> ScriptUsage {
        let names = &self.program.names;
        let mut u = ScriptUsage::default();
        for block in &self.program.blocks {
            for op in &block.ops {
                match *op {
                    Op::Load(i) => {
                        u.reads.insert(names.vars.name(i).to_owned());
                    }
                    Op::Store(i) => {
                        u.writes.insert(names.vars.name(i).to_owned());
                    }
                    Op::LoadStr(i) => {
                        u.string_reads.insert(names.strvars.name(i).to_owned());
                    }
                    Op::StoreStr(i) => {
                        u.string_writes.insert(names.strvars.name(i).to_owned());
                    }
                    Op::Sys(i) => {
                        u.system_reads.insert(names.sys.name(i).to_owned());
                    }
                    Op::External(i) => {
                        u.external.insert(names.externals.name(i).to_owned());
                    }
                    _ => {}
                }
            }
        }
        u
    }
}
