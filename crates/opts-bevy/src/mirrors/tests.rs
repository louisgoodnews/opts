//! Unit tests of `mirrors`.

use super::*;

use support::{close2, three_mirrors, Scene};

// ----- the maths ------------------------------------------------------------------------------

mod geometry;
use geometry::close;

mod cameras;
mod frames;
mod probe;
mod support;

mod ambient;
mod groups;
mod schedule;
mod tags;
mod virtual_view;
