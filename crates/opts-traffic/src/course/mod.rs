//! The course of a timetable trip on the map (M-0105): the element index that places every
//! timetable ID on a tile, and the resolver that turns a trip into lanes of the lane graph.
//!
//! Design: `docs/src/architecture/minimap-path-recognition.md`. Pure: no Bevy, no I/O; the host
//! supplies parsed tiles and the parsed `TTData`.

mod index;
mod resolve;

pub use index::{ElementCache, ElementIndex, ElementKind, ElementRef, Lookup, TileKey};
pub use resolve::{
    course_from_trip, CourseDraft, CourseGap, CourseLane, Resolution, ResolvedLane,
};
