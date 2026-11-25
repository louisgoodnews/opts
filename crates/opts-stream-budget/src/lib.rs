//! Frame-budget and prioritisation helpers for tile streaming (M-0089).
//!
//! Pure functions and small types without Bevy dependencies, so the streamer in `omsi_world`
//! (owned by the terrain/water work, see `docs/src/architecture/streaming-performance.md`) can adopt
//! them without this module touching those files, and a synthetic multi-tile test can measure the
//! work per frame.
//!
//! * [`FrameBudget`]: wall-clock budget of one frame; work items are started while it is not
//!   exhausted (at least one item per frame, so progress is guaranteed).
//! * [`wanted_tiles`]: the tiles a streamer should have resident, nearest and most-in-view first,
//!   with an optional look-ahead along the direction of travel.
//! * [`evictable`]: resident tiles beyond the radius plus hysteresis.
//! * [`Spread`]: queue that hands out at most `per_frame` items (despawn spreading).

mod cap;
mod frame;
pub mod motion;
pub mod order;
pub mod policy;
mod spread;
mod wanted;

#[cfg(test)]
mod hybrid_tests;
#[cfg(test)]
mod tests;

pub use cap::{evict_keys, over_cap, resident_cap};
pub use frame::FrameBudget;
pub use motion::Motion;
pub use spread::Spread;
pub use wanted::{
    chebyshev, evictable, priority, priority_weighted, wanted_tiles, wanted_tiles_weighted, Frame,
};

/// Integer tile key `(x, y)`; `y` grows north.
pub type TileKey = (i32, i32);
