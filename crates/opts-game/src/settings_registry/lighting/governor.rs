//! The governor setting: the frame-rate target of the automatic light governor.

use super::list::int;
use super::*;

/// The governor descriptors (registered through `lighting/list.rs`).
pub(super) fn descriptors() -> Vec<D> {
    governor()
}

/// The frame-rate target of the governor.
fn governor() -> Vec<D> {
    vec![d(
        TARGET_FPS_KEY,
        int(30, 240, 10),
        "60",
        111,
        "set-light-governor-fps",
        "set-light-governor-fps-desc",
        &[
            "governor",
            "target",
            "frame rate",
            "fps",
            "automatic",
            "vsync",
        ],
    )]
}
