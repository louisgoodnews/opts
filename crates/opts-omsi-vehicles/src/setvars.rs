//! The setvars of one vehicle (M-0078): which equipment options it offers, of what kind, with
//! which values, and what they are called. See `docs/src/formats/text/vehicles/setvars.md`.
//!
//! The catalogue merges three kinds of evidence:
//! 1. **Observed:** the `[setvar]` lines of the vehicle's repaints (`.cti`): the name, the values
//!    repaints give it (a lower bound of the range) and each repaint's own value.
//! 2. **Declared:** the vehicle's `[varnamelist]` files: a name is only offered when the vehicle
//!    declares it (scripts read it); an unknown declaration (no lists given) offers everything.
//! 3. **Documented:** the vendor's setvar document, restated in `data/setvar_families.json`
//!    (names, short meanings, value ranges, groups), matched through the vehicle folder.
//!
//! Pure data: no file access; callers read the files and pass the parsed pieces.

mod catalogue;
mod curated;
mod entry;
mod labels;
mod model;

pub use catalogue::SetvarCatalogue;
pub use curated::{curated_families, family_of_folder, parse_range, CuratedFamily, CuratedSetvar};
pub use labels::humanise;
pub use model::{Evidence, Respawn, SetvarEntry, SetvarKind, NUMBER_MAX};

#[cfg(test)]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
use crate::formats::cti::CtiSetvar;

#[cfg(test)]
use labels::labels;

#[cfg(test)]
mod tests;
