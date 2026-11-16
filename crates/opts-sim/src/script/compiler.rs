//! Compiles `.osc` token streams into bytecode blocks (`docs/src/architecture/script-vm.md`).

use super::lexer::{tokenize, Token};

mod bytecode;
pub use bytecode::{
    BinOp, Block, Diagnostic, EventKind, Names, Op, Program, Severity, StrOp, UnOp,
};

struct IfFrame {
    jump_if_zero: usize,
    else_jump: Option<usize>,
    /// Jumps of further `{else}` of the same `{if}` (broken scripts have them); they end the
    /// branch before them and the code after them never runs.
    extra_else: Vec<usize>,
    line: usize,
}

struct Open {
    name: String,
    kind: Kind,
    ops: Vec<Op>,
    ifs: Vec<IfFrame>,
    failed: bool,
}

#[derive(Clone, Copy)]
enum Kind {
    Init,
    Frame,
    FrameAi,
    Macro(u32),
    Trigger(u32),
}

/// Compiles one or more script files into a [`Program`].
#[derive(Default)]
pub struct Compiler {
    program: Program,
    diagnostics: Vec<Diagnostic>,
}

impl Compiler {
    /// Creates an empty compiler.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Compiles `source` (the text of `file`) and adds its blocks.
    pub fn add_source(&mut self, file: &str, source: &str) {
        let mut open: Option<Open> = None;
        for tok in tokenize(source) {
            if !tok.is_string && tok.text.starts_with('{') && tok.text.ends_with('}') {
                self.header(file, &tok, &mut open);
            } else if let Some(block) = open.as_mut() {
                self.token(file, &tok, block);
            } else {
                self.warn(
                    file,
                    tok.line,
                    format!("`{}` outside any block ignored", tok.text),
                );
            }
        }
        if let Some(block) = open.take() {
            self.warn(file, 0, format!("block `{}` has no {{end}}", block.name));
            self.close(file, 0, block);
        }
    }

    /// Finishes compilation; returns the program and every diagnostic.
    #[must_use]
    pub fn finish(self) -> (Program, Vec<Diagnostic>) {
        (self.program, self.diagnostics)
    }

    fn warn(&mut self, file: &str, line: usize, message: String) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            file: file.to_owned(),
            line,
            message,
        });
    }

    fn error(&mut self, file: &str, line: usize, message: String) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            file: file.to_owned(),
            line,
            message,
        });
    }
}

/// Number of registers.
pub const REGISTERS: usize = 32;

mod control;
mod header;
mod token;
mod words;
