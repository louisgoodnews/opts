//! Compiles `.osc` token streams into bytecode blocks (`docs/src/architecture/script-vm.md`).

use std::collections::HashMap;

use super::lexer::{tokenize, Token};
use super::variables::Interner;

/// Binary float operators: pop `b`, pop `a`, push `a op b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/` (division by zero yields 0)
    Div,
    /// `%` (modulo; modulo by zero yields 0)
    Mod,
    /// `min`
    Min,
    /// `max`
    Max,
    /// `>`
    Gt,
    /// `<`
    Lt,
    /// `=`
    Eq,
    /// `>=`
    Ge,
    /// `<=`
    Le,
    /// `&&`
    And,
    /// `||`
    Or,
}

/// Unary float operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// `!`
    Not,
    /// `abs`
    Abs,
    /// `sqr` (x²)
    Sqr,
    /// `sqrt`
    Sqrt,
    /// `exp` (eˣ)
    Exp,
    /// `trunc` (towards zero)
    Trunc,
    /// `/-/` (negate)
    Neg,
    /// `sin` (radians)
    Sin,
    /// `sgn`
    Sgn,
    /// `cos` (radians)
    Cos,
    /// `tan` (radians)
    Tan,
    /// `arcsin`
    Asin,
    /// `arccos`
    Acos,
    /// `arctan`
    Atan,
}

/// String-stack operators (`$…`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrOp {
    /// `$+`
    Concat,
    /// `$=`
    Eq,
    /// `$<`
    Lt,
    /// `$>`
    Gt,
    /// `$<=`
    Le,
    /// `$>=`
    Ge,
    /// `$d`
    Dup,
    /// `$*` (string, count → repeated string)
    Repeat,
    /// `$length`
    Length,
    /// `$SetLengthL` (keep the first n characters, pad right)
    SetLengthL,
    /// `$SetLengthC` (centre in n characters)
    SetLengthC,
    /// `$SetLengthR` (keep the last n characters, pad left)
    SetLengthR,
    /// `$RemoveSpaces`
    RemoveSpaces,
    /// `$IntToStr`
    IntToStr,
    /// `$IntToStrEnh` (number, format → string; format = pad character + width)
    IntToStrEnh,
    /// `$StrToFloat`
    StrToFloat,
    /// `$cutBegin`
    CutBegin,
    /// `$cutEnd`
    CutEnd,
    /// `$msg` (top string to the debug console)
    Msg,
}

/// Where an event raised with `(T.x.name)` goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// `(T.L.name)`: this vehicle (sound triggers, other scripts).
    Local,
    /// `(T.F.name)`: towards the front (announcements to the sound engine).
    Front,
    /// `(T.S.name)`: seen in a handful of files; meaning unverified.
    Sound,
}

/// One bytecode instruction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    /// Push a number.
    Push(f32),
    /// Push string-pool entry.
    PushStr(u32),
    /// `d`: duplicate the top number.
    Dup,
    /// `pi`
    PushPi,
    /// `random`: push a pseudo-random number in `[0, 1)`.
    Random,
    /// Binary float operator.
    Bin(BinOp),
    /// Unary float operator.
    Un(UnOp),
    /// String operator.
    Str(StrOp),
    /// `(L.L.x)`
    Load(u32),
    /// `(S.L.x)`
    Store(u32),
    /// `(L.$.x)`
    LoadStr(u32),
    /// `(S.$.x)`
    StoreStr(u32),
    /// `(C.L.x)`
    Const(u32),
    /// `(F.L.x)`
    Curve(u32),
    /// `(L.S.x)`
    Sys(u32),
    /// `l0`…`l7`
    LoadReg(u8),
    /// `s0`…`s7`
    StoreReg(u8),
    /// `(M.L.x)`
    Call(u32),
    /// `(M.V.x)`: macro of a vehicle-external system.
    External(u32),
    /// `(T.?.x)`
    Event(EventKind, u32),
    /// `{if}`: pop, jump to the target when zero.
    JumpIfZero(u32),
    /// `{else}`: jump to the target.
    Jump(u32),
}

/// A compiled block (`{init}`, `{frame}`, a macro or a trigger).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Display name (`init`, `frame`, `macro:name`, `trigger:name`).
    pub name: String,
    /// Source file the block came from.
    pub file: String,
    /// Instructions.
    pub ops: Vec<Op>,
}

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The construct is ignored or substituted; execution continues.
    Warning,
    /// The block is disabled.
    Error,
}

/// A compile- or run-time message with its source position.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// Severity.
    pub severity: Severity,
    /// Script file (empty for run-time messages without one).
    pub file: String,
    /// Line (0 when unknown).
    pub line: usize,
    /// Human-readable message.
    pub message: String,
}

/// All interned names of a vehicle's scripts.
#[derive(Debug, Clone, Default)]
pub struct Names {
    /// Float variables.
    pub vars: Interner,
    /// String variables.
    pub strvars: Interner,
    /// Scalar constants.
    pub consts: Interner,
    /// Curves.
    pub curves: Interner,
    /// System values.
    pub sys: Interner,
    /// Macros.
    pub macros: Interner,
    /// Events and triggers.
    pub events: Interner,
    /// External macros.
    pub externals: Interner,
}

/// The compiled scripts of one vehicle.
#[derive(Debug, Clone, Default)]
pub struct Program {
    /// All blocks.
    pub blocks: Vec<Block>,
    /// `{init}` blocks in file order.
    pub init: Vec<usize>,
    /// `{frame}` blocks in file order.
    pub frame: Vec<usize>,
    /// `{frame_ai}` blocks in file order.
    pub frame_ai: Vec<usize>,
    /// Macro id → block (later definitions replace earlier ones).
    pub macros: HashMap<u32, usize>,
    /// Event id → trigger blocks.
    pub triggers: HashMap<u32, Vec<usize>>,
    /// String literals.
    pub strings: Vec<String>,
    /// Interned names.
    pub names: Names,
}

struct IfFrame {
    jump_if_zero: usize,
    else_jump: Option<usize>,
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
                _ => self.warn(file, line, "{else} without a matching {if} ignored".into()),
            },
            _ => match block.ifs.pop() {
                Some(f) => match f.else_jump {
                    Some(j) => block.ops[j] = Op::Jump(here as u32),
                    None => block.ops[f.jump_if_zero] = Op::JumpIfZero(here as u32),
                },
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

fn word_op(word: &str) -> Option<Op> {
    use BinOp as B;
    use StrOp as S;
    use UnOp as U;
    let bytes = word.as_bytes();
    if bytes.len() == 2
        && (bytes[0] == b's' || bytes[0] == b'l')
        && (b'0'..=b'7').contains(&bytes[1])
    {
        let r = bytes[1] - b'0';
        return Some(if bytes[0] == b's' {
            Op::StoreReg(r)
        } else {
            Op::LoadReg(r)
        });
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
        "$setlengthc" => Op::Str(S::SetLengthC),
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
