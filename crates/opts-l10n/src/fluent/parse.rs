//! Parsing logic for Fluent FTL files.

use super::{Diagnostic, Part, Pattern, Table, LBRACE, RBRACE};

mod entries;
mod patterns;
mod scan;

pub use entries::*;
pub use patterns::*;
use scan::*;
