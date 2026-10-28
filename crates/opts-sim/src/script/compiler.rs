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

    fn header(&mut self, file: &str, tok: &Token, open: &mut Option<Open>) {
        let inner = tok.text[1..tok.text.len() - 1].to_lowercase();
        match inner.as_str() {
            "if" | "else" | "endif" => {
                if let Some(block) = open.as_mut() {
                    self.control(file, tok.line, &inner, block);
                } else {
                    self.warn(
                        file,
                        tok.line,
                        format!("{{{inner}}} outside any block ignored"),
                    );
                }
                return;
            }
            "end" => {
                match open.take() {
                    Some(block) => self.close(file, tok.line, block),
                    None => self.warn(file, tok.line, "{end} without a block".into()),
                }
                return;
            }
            _ => {}
        }
        if let Some(block) = open.take() {
            self.warn(
                file,
                tok.line,
                format!("block `{}` has no {{end}}", block.name),
            );
            self.close(file, tok.line, block);
        }
        let (kind, name) = match inner.split_once(':') {
            None if inner == "init" => (Kind::Init, "init".to_owned()),
            None if inner == "frame" => (Kind::Frame, "frame".to_owned()),
            None if inner == "frame_ai" => (Kind::FrameAi, "frame_ai".to_owned()),
            Some(("macro", n)) => (
                Kind::Macro(self.program.names.macros.intern(n.trim())),
                format!("macro:{}", n.trim()),
            ),
            Some(("trigger", n)) => (
                Kind::Trigger(self.program.names.events.intern(n.trim())),
                format!("trigger:{}", n.trim()),
            ),
            _ => {
                self.warn(
                    file,
                    tok.line,
                    format!("unknown block header `{}`", tok.text),
                );
                return;
            }
        };
        *open = Some(Open {
            name,
            kind,
            ops: Vec::new(),
            ifs: Vec::new(),
            failed: false,
        });
    }

    fn control(&mut self, file: &str, line: usize, word: &str, block: &mut Open) {
        let here = block.ops.len();
        match word {
            "if" => {
                block.ops.push(Op::JumpIfZero(0));
                block.ifs.push(IfFrame {
                    jump_if_zero: here,
                    else_jump: None,
                    extra_else: Vec::new(),
                    line,
                });
            }
            "else" => match block.ifs.last_mut() {
                Some(f) if f.else_jump.is_none() => {
                    f.else_jump = Some(here);
                    let jz = f.jump_if_zero;
                    block.ops.push(Op::Jump(0));
                    block.ops[jz] = Op::JumpIfZero(here as u32 + 1);
                }
                Some(f) => {
                    // A second `{else}` (`{if} A {else} B {else} C {endif}`): OMSI's interpreter
                    // runs B, skips to `{endif}` at the second `{else}` and never runs C.
                    f.extra_else.push(here);
                    block.ops.push(Op::Jump(0));
                    self.warn(
                        file,
                        line,
                        "second {else} of one {if}: its branch never runs".into(),
                    );
                }
                None => self.warn(file, line, "{else} without a matching {if} ignored".into()),
            },
            _ => match block.ifs.pop() {
                Some(f) => {
                    for j in &f.extra_else {
                        block.ops[*j] = Op::Jump(here as u32);
                    }
                    match f.else_jump {
                        Some(j) => block.ops[j] = Op::Jump(here as u32),
                        None => block.ops[f.jump_if_zero] = Op::JumpIfZero(here as u32),
                    }
                }
                None => self.warn(file, line, "{endif} without a matching {if} ignored".into()),
            },
        }
    }

    fn close(&mut self, file: &str, line: usize, mut block: Open) {
        // OMSI tolerates a block that ends inside open conditionals (60+ vehicle scripts do):
        // the open branches simply end with the block.
        while let Some(f) = block.ifs.pop() {
            self.warn(
                file,
                f.line,
                format!(
                    "{{if}} without {{endif}} in `{}` closed at {{end}}",
                    block.name
                ),
            );
            let end = block.ops.len() as u32;
            for j in &f.extra_else {
                block.ops[*j] = Op::Jump(end);
            }
            match f.else_jump {
                Some(j) => block.ops[j] = Op::Jump(end),
                None => block.ops[f.jump_if_zero] = Op::JumpIfZero(end),
            }
        }
        if block.failed {
            self.error(file, line, format!("block `{}` disabled", block.name));
            return;
        }
        let index = self.program.blocks.len();
        self.program.blocks.push(Block {
            name: block.name,
            file: file.to_owned(),
            ops: block.ops,
        });
        match block.kind {
            Kind::Init => self.program.init.push(index),
            Kind::Frame => self.program.frame.push(index),
            Kind::FrameAi => self.program.frame_ai.push(index),
            Kind::Macro(id) => {
                self.program.macros.insert(id, index);
            }
            Kind::Trigger(id) => self.program.triggers.entry(id).or_default().push(index),
        }
    }

    fn token(&mut self, file: &str, tok: &Token, block: &mut Open) {
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

    fn accessor(&mut self, inner: &str) -> Option<Op> {
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

/// Number of registers.
pub const REGISTERS: usize = 32;

fn word_op(word: &str) -> Option<Op> {
    use BinOp as B;
    use StrOp as S;
    use UnOp as U;
    let bytes = word.as_bytes();
    // Registers `s0`…`s31` / `l0`…`l31` (the corpus uses up to `s9`; the documented eight are
    // not the limit of OMSI's scripts).
    if (bytes.len() == 2 || bytes.len() == 3)
        && (bytes[0] == b's' || bytes[0] == b'l')
        && bytes[1..].iter().all(u8::is_ascii_digit)
    {
        if let Ok(r) = word[1..].parse::<u8>() {
            if r < REGISTERS as u8 {
                return Some(if bytes[0] == b's' {
                    Op::StoreReg(r)
                } else {
                    Op::LoadReg(r)
                });
            }
        }
    }
    Some(match word {
        "+" => Op::Bin(B::Add),
        "-" => Op::Bin(B::Sub),
        "*" => Op::Bin(B::Mul),
        "/" => Op::Bin(B::Div),
        "%" => Op::Bin(B::Mod),
        "min" => Op::Bin(B::Min),
        "max" => Op::Bin(B::Max),
        ">" => Op::Bin(B::Gt),
        "<" => Op::Bin(B::Lt),
        "=" => Op::Bin(B::Eq),
        ">=" => Op::Bin(B::Ge),
        "<=" => Op::Bin(B::Le),
        "&&" => Op::Bin(B::And),
        "||" => Op::Bin(B::Or),
        "!" => Op::Un(U::Not),
        "abs" => Op::Un(U::Abs),
        "sqr" => Op::Un(U::Sqr),
        "sqrt" => Op::Un(U::Sqrt),
        "exp" => Op::Un(U::Exp),
        "trunc" => Op::Un(U::Trunc),
        "/-/" => Op::Un(U::Neg),
        "sin" => Op::Un(U::Sin),
        "sgn" => Op::Un(U::Sgn),
        "cos" => Op::Un(U::Cos),
        "tan" => Op::Un(U::Tan),
        "arcsin" => Op::Un(U::Asin),
        "arccos" => Op::Un(U::Acos),
        "arctan" => Op::Un(U::Atan),
        "d" => Op::Dup,
        "pi" => Op::PushPi,
        "random" => Op::Random,
        "$+" => Op::Str(S::Concat),
        "$=" => Op::Str(S::Eq),
        "$<" => Op::Str(S::Lt),
        "$>" => Op::Str(S::Gt),
        "$<=" => Op::Str(S::Le),
        // `$=>` is a frequent typo of `$>=` in add-on scripts (52 uses); read as `$>=`.
        "$>=" | "$=>" => Op::Str(S::Ge),
        "$d" => Op::Str(S::Dup),
        "$*" => Op::Str(S::Repeat),
        "$length" => Op::Str(S::Length),
        "$setlengthl" => Op::Str(S::SetLengthL),
        // `$SetLengthM` (mittig) appears in the BVG ATRON scripts: the centred variant.
        "$setlengthc" | "$setlengthm" => Op::Str(S::SetLengthC),
        "$setlengthr" => Op::Str(S::SetLengthR),
        "$removespaces" => Op::Str(S::RemoveSpaces),
        "$inttostr" => Op::Str(S::IntToStr),
        "$inttostrenh" => Op::Str(S::IntToStrEnh),
        "$strtofloat" => Op::Str(S::StrToFloat),
        "$cutbegin" => Op::Str(S::CutBegin),
        "$cutend" => Op::Str(S::CutEnd),
        "$msg" => Op::Str(S::Msg),
        _ => return None,
    })
}
