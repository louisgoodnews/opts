//! missing-id warnings, external macros, events and the random generator.

use super::*;

impl ScriptEngine {
    /// Reports an undefined constant (0), curve (1) or macro (2) once per name.
    pub(super) fn warn_missing(&mut self, what: u8, id: u32) {
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

    /// Calls the host's implementation of callback `id`. A callback the host does not implement
    /// is answered by the stub of its signature: the arguments leave the stacks and neutral
    /// results arrive, so the script's later operations stay aligned (an unimplemented callback
    /// that left its arguments behind corrupted every later operation of the block). It is
    /// reported once per engine and counted in [`ScriptEngine::callback_stats`].
    pub(super) fn call_external(&mut self, id: u32, st: &mut Stacks) {
        let name = self.program.names.externals.name(id).to_owned();
        let handled = match self.external.as_mut() {
            Some(handler) => handler(&name, st),
            None => false,
        };
        if handled {
            *self.callback_stats.real.entry(name).or_default() += 1;
            return;
        }
        let why = if self.external.is_some() {
            "is not implemented by the host"
        } else {
            "has no handler"
        };
        match crate::script::callbacks::signature(&name) {
            Some(sig) => {
                sig.apply_stub(st);
                *self.callback_stats.stub.entry(name.clone()).or_default() += 1;
                self.warn_once(
                    format!("external {name}"),
                    format!("callback `{name}` {why}; answered with a neutral stub"),
                );
            }
            None => {
                *self.callback_stats.unknown.entry(name.clone()).or_default() += 1;
                self.warn_once(
                    format!("external {name}"),
                    format!("external macro `{name}` {why} and is not a known callback; ignored"),
                );
            }
        }
    }

    pub(super) fn raise(&mut self, kind: EventKind, id: u32) {
        let name = self.program.names.events.name(id).to_owned();
        self.events.push(crate::script::ScriptEvent { kind, name });
    }

    pub(super) fn next_random(&mut self) -> f32 {
        // xorshift32: deterministic for a given seed (replays).
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / 16_777_216.0
    }
}
