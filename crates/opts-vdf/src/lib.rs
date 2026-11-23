//! Valve's KeyValues text (VDF): the tokenizer and parser behind Steam's `libraryfolders.vdf`
//! and `appmanifest_*.acf` files. Pure Rust with no dependencies: no file access, no OMSI and no
//! engine types.

mod kv;

#[cfg(test)]
mod tests;

pub use kv::{parse, Kv};
