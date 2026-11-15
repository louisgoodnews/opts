//! tokens of a block body and the register and accessor words.

use super::*;
use super::words::word_op;

impl Compiler {

    pub(super) fn token(&mut self, file: &str, tok: &Token, block: &mut Open) {
        if tok.is_string {
            let id = self.program.strings.len() as u32;
            self.program.strings.push(tok.text.clone());
            block.ops.push(Op::PushStr(id));
            return;
        }
        let word = tok.text.to_lowercase();
        let op = if let Ok(n) = tok.text.parse::<f32>() {
            Some(Op::Push(n))
        } else if tok.text.starts_with('(') && tok.text.ends_with(')') {
            self.accessor(&tok.text[1..tok.text.len() - 1])
        } else {
            word_op(&word)
        };
        match op {
            Some(op) => block.ops.push(op),
            None => self.warn(
                file,
                tok.line,
                format!("unknown token `{}` ignored", tok.text),
            ),
        }
    }

    pub(super) fn accessor(&mut self, inner: &str) -> Option<Op> {
        let mut parts = inner.splitn(3, '.');
        let (op, scope, name) = (parts.next()?, parts.next()?, parts.next()?);
        let n = &mut self.program.names;
        Some(
            match (op.to_lowercase().as_str(), scope.to_lowercase().as_str()) {
                ("l", "l") => Op::Load(n.vars.intern(name)),
                ("s", "l") => Op::Store(n.vars.intern(name)),
                ("l", "$") => Op::LoadStr(n.strvars.intern(name)),
                ("s", "$") => Op::StoreStr(n.strvars.intern(name)),
                ("c", "l") => Op::Const(n.consts.intern(name)),
                ("f", "l") => Op::Curve(n.curves.intern(name)),
                ("l", "s") => Op::Sys(n.sys.intern(name)),
                ("m", "l") => Op::Call(n.macros.intern(name)),
                ("m", "v") => Op::External(n.externals.intern(name)),
                ("t", "l") => Op::Event(EventKind::Local, n.events.intern(name)),
                ("t", "f") => Op::Event(EventKind::Front, n.events.intern(name)),
                ("t", "s") => Op::Event(EventKind::Sound, n.events.intern(name)),
                _ => return None,
            },
        )
    }

}
