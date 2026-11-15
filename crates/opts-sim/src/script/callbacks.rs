//! The engine callbacks scripts reach with `(M.V.name)`: their stack signatures, the host
//! handler interface and the stack-correct stub for every callback the host does not implement
//! (`docs/src/architecture/osc-runtime.md`, "The Host Interface").
//!
//! The registry, not the host, moves the stacks: the VM pops the arguments a signature names,
//! hands them to the handler and pushes exactly the results the signature names. A callback the
//! host does not implement therefore never misaligns the stacks; it answers with neutral values
//! and is reported once per engine. Signatures come from the callbacks reference
//! (`docs/src/formats/text/vehicles/callbacks.md`) and OMSI's shipped
//! `program/callbacklist_*.txt`.

mod args;
mod handler;
mod signature;
mod table;
#[cfg(test)]
mod tests;

pub use args::{CallbackArgs, CallbackResult};
pub use handler::{CallbackHandler, CallbackStats};
pub use signature::{CallbackSignature, Neutral};
pub use table::{signature, CALLBACKS};
