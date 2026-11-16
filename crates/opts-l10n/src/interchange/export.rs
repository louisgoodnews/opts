//! Exporting reference and translation tables to JSON (`opts-l10n/1`) and `.ftl` templates.

use super::*;

mod format;
mod ftl;
mod json;

use format::*;
pub use ftl::*;
pub use json::*;
