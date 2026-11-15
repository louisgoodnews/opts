//! Bytecode execution of the script engine.

use super::compiler::{BinOp, Diagnostic, EventKind, Op, Severity, UnOp};
use super::strings::{self, Stacks};
use super::ScriptEngine;

/// Deepest macro nesting (OMSI scripts have no loops, so only recursion could run away).
const MAX_DEPTH: usize = 64;
/// Float stack depth beyond which values are dropped.
const MAX_STACK: usize = 256;

mod exec;
mod external;
mod ops;
mod step;
use ops::{binary, unary};
