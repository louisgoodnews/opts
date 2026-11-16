//! one instruction at a time: the control flow and the data operations.

use super::*;

impl ScriptEngine {
    /// Executes one instruction; returns a jump target.
    pub(super) fn step(
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
            // `{if}` looks at the condition and leaves it on the stack, like a store leaves its
            // value: the scripts' own idioms need it (`(L.L.elec_busbar_main) {if}
            // (L.L.Drucker_an) ! = {if}` compares the busbar still on the stack with the negated
            // state; the NLC's ticket printer never starts without it). Evidence in
            // `docs/src/architecture/script-vm.md`.
            Op::JumpIfZero(t) => {
                if st.numbers.last().copied().unwrap_or(0.0) == 0.0 {
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
    pub(super) fn step_data(
        &mut self,
        op: Op,
        depth: usize,
        st: &mut Stacks,
        block: usize,
        pc: usize,
    ) {
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
}
