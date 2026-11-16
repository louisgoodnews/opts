//! JSON-RPC 2.0 types of the OPTS control protocol (`docs/src/architecture/control-protocol.md`).
//!
//! One message per line. The same types are used by the client in this crate and by the server in
//! `opts-bevy::control`.

/// Version reported by the `version` method; additive changes bump the minor part.
pub const PROTOCOL_VERSION: &str = "1.36.0";
/// Port the control server uses when it is started without an explicit one.
pub const DEFAULT_PORT: u16 = 44201;
/// Longest accepted line in bytes.
pub const MAX_LINE_BYTES: usize = 1 << 20;
/// Most commands one `batch` may carry.
pub const MAX_BATCH: usize = 100;

/// Error codes (JSON-RPC reserved range and application range).
pub mod code;
mod message;
mod parse;

pub use message::{Request, Response, RpcError};
pub use parse::{parse_line, parse_response_line, Incoming};

#[cfg(test)]
mod tests;
