//! Spline attachments and attached objects of a tile (`[splineAttachement*]`, `[attachObj]`).

/// Object attached to a spline (`[splineAttachement]`) or continued from another tile
/// (`[splineAttachement_repeater]`): street furniture, fences, shrubs, parking bays.
///
/// Layout (version 14, counted over the installed corpus; `docs/src/formats/map/tiles.md`): flag,
/// [repeater only: master tile index, first instance index], `.sco`, ID, spline index, lateral
/// offset, height, distance, heading, `field8` (always `0`), `field9`, spacing, range, `field12`,
/// string count, strings. Version 10 has no `field8`, `field9` and `field12`.
///
/// An attachment stands for the instances `k` of [`Self::instances`] at the chainages
/// [`Self::chainage`]`(k) = distance + k · spacing`, measured from the start of the connected spline
/// chain the carrier belongs to (walk `conn_id_1` backwards, summing lengths). A master places the
/// instances that fall on its own tile; a repeater continues the same set on another tile from
/// instance [`RepeaterLink::first_instance`] on.
#[derive(Debug, Clone, PartialEq)]
pub struct MapSplineAttachment {
    /// Attached scenery object `.sco` path.
    pub sco_file: String,
    /// Attachment object ID code.
    pub id_code: u64,
    /// Index of the carrying spline among this tile's `[spline]` blocks.
    pub spline_index: usize,
    /// Lateral offset from the spline centreline in metres (positive = right of travel).
    pub lateral_m: f32,
    /// Height above the spline surface in metres.
    pub height_m: f32,
    /// Chainage of instance 0 in metres, from the start of the carrier's spline chain.
    pub distance_m: f32,
    /// Heading relative to the spline direction in degrees.
    pub heading_deg: f32,
    /// Unidentified field after the heading (always `0` in the corpus; `0` in version 10).
    pub field8: f32,
    /// Unidentified field (`0` or ±2–5 m in the corpus; `0` in version 10) ❓.
    pub field9: f32,
    /// Distance between consecutive instances in metres.
    pub spacing_m: f32,
    /// Length of the run of instances in metres (`0` = a single instance).
    pub range_m: f32,
    /// Unidentified field (`0` or `1` in the corpus; `0` in version 10) ❓.
    pub field12: f32,
    /// What continues a master of another tile (`[splineAttachement_repeater]` only).
    pub repeater: Option<RepeaterLink>,
    /// Per-instance string variables (blank values kept), as for `[object]`.
    pub string_vars: Vec<String>,
}

/// The two leading fields of `[splineAttachement_repeater]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepeaterLink {
    /// Index of the master's tile in the `global.cfg` `[map]` list (not needed for placement).
    pub master_tile: i64,
    /// Index of the first instance that falls on this repeater's spline.
    pub first_instance: u32,
}

impl MapSplineAttachment {
    /// The instance indices this record places: from `0` (master) or the repeater's first
    /// instance to `⌊range / spacing⌋`; a single instance when spacing or range is not positive.
    #[must_use]
    pub fn instances(&self) -> std::ops::RangeInclusive<u32> {
        let first = self.repeater.map_or(0, |r| r.first_instance);
        if self.spacing_m > 0.0 && self.range_m > 0.0 {
            // A small allowance: `range` is written as a multiple of `spacing` in floating point.
            let last = (f64::from(self.range_m) / f64::from(self.spacing_m) + 1e-4).floor();
            first..=(last.min(f64::from(u32::MAX)) as u32)
        } else {
            first..=first
        }
    }

    /// Whether the record is a run of instances (positive spacing and range).
    #[must_use]
    pub fn is_run(&self) -> bool {
        self.spacing_m > 0.0 && self.range_m > 0.0
    }

    /// Chainage of instance `k` from the start of the carrier's spline chain (metres).
    #[must_use]
    pub fn chainage(&self, k: u32) -> f64 {
        f64::from(self.distance_m) + f64::from(k) * f64::from(self.spacing_m.max(0.0))
    }
}

/// Object placed on an attachment point of another object (`[attachObj]`): line modules and
/// timetables on bus-stop signs, signal heads on their arms, plates on sign poles. 62,609 records
/// in the installed corpus; every parent lies on the same tile (an `[object]`, another
/// `[attachObj]` or a spline attachment). Layout (counted over the corpus): `flag`, `.sco`, ID, parent
/// ID, a field that is always `0`, the parent's attachment point index (0-based, file order of its
/// `[new_attachment]` blocks), heading, then in version 14 pitch and roll, then the string
/// variables as in `[object]` (count, strings).
#[derive(Debug, Clone, PartialEq)]
pub struct MapAttachedObject {
    /// Placement flag (`0`).
    pub flag: i32,
    /// Attached scenery object `.sco` path.
    pub sco_file: String,
    /// ID code of this object (other attached objects may name it as their parent).
    pub id_code: u64,
    /// ID code of the parent object.
    pub parent_id: u64,
    /// Index of the parent's attachment point.
    pub attachment: usize,
    /// Heading relative to the attachment point (degrees, clockwise like `[object]`).
    pub heading_deg: f32,
    /// Pitch (version 14 only).
    pub pitch_deg: Option<f32>,
    /// Roll (version 14 only).
    pub roll_deg: Option<f32>,
    /// Per-object string variables.
    pub string_vars: Vec<String>,
}
