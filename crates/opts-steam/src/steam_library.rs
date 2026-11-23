//! Finding OMSI 2 through Steam's library configuration
//! (`docs/src/architecture/application-startup.md`, "Finding OMSI through Steam").
//!
//! Steam lists its libraries in `steamapps/libraryfolders.vdf` and names a game's folder in
//! `appmanifest_<appid>.acf`, both in Valve's KeyValues text format. OMSI 2 is app 252530. Only
//! reads; a missing or unreadable file is skipped.

/// The Steam app id of OMSI 2.
pub const OMSI2_APP_ID: &str = "252530";

/// The folder Steam uses for OMSI 2 when its manifest is missing.
const DEFAULT_INSTALL_DIR: &str = "OMSI 2";

mod libraries;
mod roots;

pub use libraries::{find_omsi, install_dir, libraries, Library};
pub use opts_vdf::{parse, Kv};
pub use roots::steam_roots;

#[cfg(test)]
mod tests;
