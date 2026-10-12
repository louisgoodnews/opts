//! Bytecode execution of the script engine.

use super::compiler::{BinOp, Diagnostic, EventKind, Op, Severity, UnOp};
use super::strings::{self, Stacks};
use super::ScriptEngine;

/// Deepest macro nesting (OMSI scripts have no loops, so only recursion could run away).
const MAX_DEPTH: usize = 64;
/// Float stack depth beyond which values are dropped.
const MAX_STACK: usize = 256;

fn truth(b: bool) -> f32 {
    f32::from(u8::from(b))
}

fn binary(op: BinOp, a: f32, b: f32) -> f32 {
    match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => {
            if b == 0.0 {
                0.0
            } else {
                a / b
            }
        }
        BinOp::Mod => {
            if b == 0.0 {
                0.0
            } else {
                a % b
            }
        }
        BinOp::Min => a.min(b),
        BinOp::Max => a.max(b),
        BinOp::Gt => truth(a > b),
        BinOp::Lt => truth(a < b),
        BinOp::Eq => truth(a == b),
        BinOp::Ge => truth(a >= b),
        BinOp::Le => truth(a <= b),
        BinOp::And => truth(a != 0.0 && b != 0.0),
        BinOp::Or => truth(a != 0.0 || b != 0.0),
    }
}

fn unary(op: UnOp, x: f32) -> f32 {
    match op {
        UnOp::Not => truth(x == 0.0),
        UnOp::Abs => x.abs(),
        UnOp::Sqr => x * x,
        UnOp::Sqrt => x.max(0.0).sqrt(),
        UnOp::Exp => x.exp(),
        UnOp::Trunc => x.trunc(),
        UnOp::Neg => -x,
        UnOp::Sin => x.sin(),
        UnOp::Cos => x.cos(),
        UnOp::Tan => x.tan(),
        UnOp::Asin => x.clamp(-1.0, 1.0).asin(),
        UnOp::Acos => x.clamp(-1.0, 1.0).acos(),
        UnOp::Atan => x.atan(),
        UnOp::Sgn => {
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            }
        }
    }
}

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
    pub(super) fn run_block(&mut self, index: usize, depth: usize, st: &mut Stacks) {
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

    fn pop(&mut self, st: &mut Stacks, block: usize, pc: usize) -> f32 {
        match st.numbers.pop() {
            Some(v) => v,
            None => {
                self.report_underflow(block, pc);
                0.0
            }
        }
    }

    fn report_underflow(&mut self, block: usize, pc: usize) {
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

    /// Executes one instruction; returns a jump target.
    fn step(
        &mut self,
        op: Op,
        block: usize,
        pc: usize,
        depth: usize,
        st: &mut Stacks,
    ) -> Option<usize> {
        match op {
            Op::Push(v) => st.numbers.push(v),
            Op::PushStr(i) => st.strings.push(self.program.strings[i as usize].clone()),
            Op::Dup => {
                let v = self.pop(st, block, pc);
                st.numbers.extend([v, v]);
            }
            Op::PushPi => st.numbers.push(std::f32::consts::PI),
            // `x random`: a pseudo-random number from 0 up to `x` (`500000 random 400000 +`).
            Op::Random => {
                let range = self.pop(st, block, pc);
                let v = self.next_random();
                st.numbers.push(v * range);
            }
            Op::Bin(b) => {
                let y = self.pop(st, block, pc);
                let x = self.pop(st, block, pc);
                st.numbers.push(binary(b, x, y));
            }
            Op::Un(u) => {
                let x = self.pop(st, block, pc);
                st.numbers.push(unary(u, x));
            }
            Op::Str(s) => {
                let mut underflow = false;
                strings::exec(s, st, &mut self.messages, &mut underflow);
                if underflow {
                    self.warn_once(
                        format!("underflow {block}:{pc}"),
                        "stack underflow in string operator".into(),
                    );
                }
            }
            Op::JumpIfZero(t) => {
                if self.pop(st, block, pc) == 0.0 {
                    return Some(t as usize);
                }
            }
            Op::Jump(t) => return Some(t as usize),
            Op::LoadReg(r) => st.numbers.push(st.regs[usize::from(r)]),
            // Like a variable store, a register store leaves the value on the stack
            // (`X s2 (L.L.y) +` adds `y` to `X`).
            Op::StoreReg(r) => {
                let v = self.pop(st, block, pc);
                st.regs[usize::from(r)] = v;
                st.numbers.push(v);
            }
            other => self.step_data(other, depth, st, block, pc),
        }
        None
    }

    /// Variable, constant, system, call and event instructions.
    fn step_data(&mut self, op: Op, depth: usize, st: &mut Stacks, block: usize, pc: usize) {
        match op {
            Op::Load(id) => st.numbers.push(self.vars.float(id)),
            // A store leaves its value on the stack: `1 (S.L.a) (S.L.b)` sets both variables
            // (the idiom runs through the original scripts; with a popping store the second
            // variable would read an empty stack).
            Op::Store(id) => {
                let v = self.pop(st, block, pc);
                self.vars.set_float(id, v);
                st.numbers.push(v);
            }
            Op::LoadStr(id) => st.strings.push(self.vars.string(id).to_owned()),
            Op::StoreStr(id) => {
                let s = st.strings.pop().unwrap_or_default();
                self.vars.set_string(id, s.clone());
                st.strings.push(s);
            }
            Op::Const(id) => {
                let v = self.resolved_consts.get(id as usize).copied().flatten();
                match v {
                    Some(v) => st.numbers.push(v),
                    None => {
                        self.warn_missing(0, id);
                        st.numbers.push(0.0);
                    }
                }
            }
            Op::Curve(id) => {
                let x = self.pop(st, block, pc);
                let y = self
                    .resolved_curves
                    .get(id as usize)
                    .and_then(Option::as_ref)
                    .map(|c| c.eval(x));
                match y {
                    Some(y) => st.numbers.push(y),
                    None => {
                        self.warn_missing(1, id);
                        st.numbers.push(0.0);
                    }
                }
            }
            Op::Sys(id) => st
                .numbers
                .push(self.sys.get(id as usize).copied().unwrap_or(0.0)),
            Op::Call(id) => match self.program.macros.get(&id).copied() {
                Some(b) => self.run_block(b, depth + 1, st),
                None => self.warn_missing(2, id),
            },
            Op::External(id) => self.call_external(id, st),
            Op::Event(kind, id) => self.raise(kind, id),
            _ => {}
        }
    }

    /// Reports an undefined constant (0), curve (1) or macro (2) once per name.
    fn warn_missing(&mut self, what: u8, id: u32) {
        if !self.missing_seen.insert((what, id)) {
            return;
        }
        let names = &self.program.names;
        let message = match what {
            0 => format!("undefined constant `{}` reads 0", names.consts.name(id)),
            1 => format!("undefined curve `{}` yields 0", names.curves.name(id)),
            _ => format!(
                "call to undefined macro `{}` ignored",
                names.macros.name(id)
            ),
        };
        self.diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            file: String::new(),
            line: 0,
            message,
        });
    }

    fn call_external(&mut self, id: u32, st: &mut Stacks) {
        let name = self.program.names.externals.name(id).to_owned();
        match self.external.as_mut() {
            Some(handler) => handler(&name, st),
            None => self.warn_once(
                format!("external {name}"),
                format!("external macro `{name}` has no handler; ignored"),
            ),
        }
    }

    pub(super) fn raise(&mut self, kind: EventKind, id: u32) {
        let name = self.program.names.events.name(id).to_owned();
        self.events.push(super::ScriptEvent { kind, name });
    }

    fn next_random(&mut self) -> f32 {
        // xorshift32: deterministic for a given seed (replays).
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / 16_777_216.0
    }
}
