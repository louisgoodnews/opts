//! Command-line surface of OPTS.
//!
//! * [`args`] — arguments of the game binary (`opts`), shared with `opts-game`.
//! * [`commands`] — the table of protocol methods (used by the client and by the server's `help`).
//! * [`protocol`] — the JSON-RPC types of the control protocol, shared by the client and the server in
//!   `opts-bevy` (see `docs/src/architecture/control-protocol.md`).
//! * With the `client` feature (default): the `opts-cli` control client — transport, high-level client,
//!   command table, output formatting and the REPL.

#[cfg(feature = "client")]
#[macro_use]
mod macros;

pub mod args;
pub mod protocol;

pub use args::{CliArgs, MIN_WINDOW_SIZE};

#[cfg(feature = "client")]
pub mod client;
pub mod commands;
pub mod reference;
#[cfg(feature = "client")]
pub mod output;
#[cfg(feature = "client")]
pub mod repl;
#[cfg(feature = "client")]
pub mod run;
#[cfg(feature = "client")]
pub mod transport;
