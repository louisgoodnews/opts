//! block execution: the stack-depth guard, warnings and the underflow report.

use super::*;

impl ScriptEngine {

    pub(super) fn warn_once(&mut self, key: String, message: String) {
        if self.warned.insert(key) {
            self.diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                file: String::new(),
                line: 0,
                message,
            });
        }
    }

    /// Runs block `index` with fresh registers; `depth` counts nested macro calls.
    pub(in crate::script) fn run_block(&mut self, index: usize, depth: usize, st: &mut Stacks) {
        if depth >= MAX_DEPTH {
            let name = self.program.blocks[index].name.clone();
            self.warn_once(
                format!("depth {name}"),
                format!("macro nesting too deep at `{name}`"),
            );
            return;
        }
        let mut pc = 0usize;
        while let Some(op) = self.program.blocks[index].ops.get(pc).copied() {
            pc += 1;
            self.ops_executed += 1;
            if let Some(target) = self.step(op, index, pc - 1, depth, st) {
                pc = target;
            }
            // Values the scripts leave behind pile up at the bottom; the oldest are dropped.
            if st.numbers.len() > 2 * MAX_STACK {
                let extra = st.numbers.len() - MAX_STACK;
                st.numbers.drain(..extra);
            }
            if st.strings.len() > 2 * MAX_STACK {
                let extra = st.strings.len() - MAX_STACK;
                st.strings.drain(..extra);
            }
        }
    }

    pub(super) fn pop(&mut self, st: &mut Stacks, block: usize, pc: usize) -> f32 {
        match st.numbers.pop() {
            Some(v) => v,
            None => {
                self.report_underflow(block, pc);
                0.0
            }
        }
    }

    pub(super) fn report_underflow(&mut self, block: usize, pc: usize) {
        let site = ((block as u32) << 16) | (pc as u32 & 0xFFFF);
        if self.missing_seen.insert((3, site)) {
            let name = &self.program.blocks[block].name;
            self.diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                file: self.program.blocks[block].file.clone(),
                line: 0,
                message: format!("stack underflow in `{name}` (op {pc}); 0 substituted"),
            });
        }
    }

}
