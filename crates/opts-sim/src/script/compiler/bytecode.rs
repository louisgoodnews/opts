//! Bytecode instructions, operators, compiled blocks, and diagnostics for `.osc` scripts.

use std::collections::HashMap;

use super::super::variables::Interner;

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
