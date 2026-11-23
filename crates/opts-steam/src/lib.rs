//! Steam library and OMSI install discovery: the library folders from `libraryfolders.vdf`, the
//! OMSI 2 folder from its app manifest, and the check that a folder is an OMSI root. Reads only.
//! The game uses it at start-up through `opts-game`'s re-exports.

pub mod omsi_root;
pub mod steam_library;
