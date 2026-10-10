//! Handlers of the control protocol, one module per capability group.

pub mod batch;
pub mod input;
pub mod meta;
pub mod output;
pub mod query;
pub mod scene;
pub mod sim;

use super::state::ControlRegistry;

/// Registers every handler implemented by `opts-bevy`. Game-specific methods are added by `opts-game`.
pub fn register_all(registry: &mut ControlRegistry) {
    meta::register(registry);
    query::register(registry);
    input::register(registry);
    sim::register(registry);
    scene::register(registry);
    output::register(registry);
    batch::register(registry);
}
