//! The operations on session vehicles: switch the active one, despawn one, put one back at its
//! spawn point, and notice vehicles that vanished. They take `&mut World` so that the pause menu,
//! the quick-switch key and the control protocol run exactly the same code, and so that a test can
//! call them without a window.


mod despawn;
mod frame;
mod respawn;
mod switch;
mod sync;

pub use despawn::{despawn, DespawnOutcome};
pub use frame::{abs_to_local, entity_of, id_of, local_to_abs, OpError, PLACE_LABEL_HIDE_DISTANCE_M};
pub use respawn::{respawn, respawn_at};
pub use switch::{switch_step, switch_to};
pub use sync::{refresh_records, sync_lost};
