//! Pure geometry: reflection, pose, field of view, pane and UV.

use super::*;

mod mirror;
mod pane;
mod visibility;

pub(super) fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-4
}
