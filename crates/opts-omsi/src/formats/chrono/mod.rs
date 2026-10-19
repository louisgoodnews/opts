//! Chrono: dated overlays on a map (`maps/<Map>/Chrono/<entry>/…`).
//!
//! An **entry** is any folder below `Chrono/` (at any depth) that holds a `Chrono.cfg`. It is
//! **active** on a date `d` when `start ≤ d ≤ end` (a missing start is "from the beginning", a
//! missing end "open-ended", the end is inclusive). Active entries are applied in the order
//! `(start date or −∞, folder path case-insensitive)`, later ones over earlier ones,
//! cumulatively:
//!
//! * tile overlays (`tile_X_Y.map`) are diffs applied to the base tile ([`TileOverlay`],
//!   [`crate::formats::map::MapTile::apply_overlay`]); overlay tiles without a base file are new
//!   tiles;
//! * `.chronoterrrelevant` markers name the tiles whose 3 × 3 neighbourhood needs a new terrain
//!   carve (raw terrain and water never change);
//! * `parklist_p*.txt`, `registrations.txt` and `unsched_trafficdens.txt` replace the whole file
//!   (the last active entry that has it wins), `TTData/` overlays file by file, and
//!   `ailists_#upd.cfg` replaces `[aigroup_2]` groups by name and `[aigroup_depot]` depots by
//!   depot name ([`merge_ailists`]).
//!
//! See `docs/src/formats/map/chrono.md`.

mod data;
mod entry;
mod overlay;

pub use data::merge_ailists;
pub use entry::{
    ActiveSet, ChronoDate, ChronoEntry, ChronoPlan, ChronoSet, ChronoSummary, ChronoSwitch,
};
pub use overlay::{OverlayOp, OverlayReport, RuleEdit, TileOverlay};
