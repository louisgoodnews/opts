//! Parsing logic for Fluent FTL files.

use super::{Diagnostic, Part, Pattern, Table, LBRACE, RBRACE};

mod scan;
mod entries;
mod patterns;

use scan::*;
pub use entries::*;
pub use patterns::*;















