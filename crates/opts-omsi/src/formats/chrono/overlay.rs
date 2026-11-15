//! Chrono tile overlays: `Chrono/<entry>/tile_X_Y.map` as a list of edits to the base tile.
//!
//! An overlay is a diff, never a copy (`docs/src/formats/map/chrono.md`). `[selobject] id` /
//! `[selspline] id` select an element; the selection holds for the edits that follow and is
//! cleared by the next selection or by any added element. Edits: `[rule]` (upsert keyed by owner,
//! path index, type and flag), `[kill_rule]` (remove by path index and type), `[relabel]` (`n`, then
//! exactly `n` literal lines, blank lines being values: replaces all string variables), `[typ]`
//! (replaces the `.sco` of an object or the `.sli` of a spline) and `[delete]`. Added elements
//! (`[object]`, `[spline]`, `[spline_h]`, `[attachObj]`, `[splineAttachement*]`, `[varparent]`,
//! `[rule]`s that follow them) use the base layouts and are parsed by [`MapTile::from_text`].
//! Terrain and water are never changed.

use crate::formats::map::MapTile;
use crate::formats::text::{decode_bytes, parse_float};

/// A `[rule]` edit of the selected element.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleEdit {
    /// Path index among the owner's paths.
    pub path_index: usize,
    /// Rule type (`trafficdensity`, `speedlimit`, …).
    pub rule_type: String,
    /// New value.
    pub value: f32,
    /// Flag (part of the key).
    pub flag: u32,
}

/// One step of an overlay, in file order.
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayOp {
    /// `[selobject]`: an `[object]`, `[attachObj]`, `[splineAttachement]` or repeater (one ID
    /// space).
    SelectObject(u64),
    /// `[selspline]`: a `[spline]` / `[spline_h]`.
    SelectSpline(u64),
    /// `[rule]` after a selection.
    Rule(RuleEdit),
    /// `[kill_rule]`: path index and type (value and flag are ignored).
    KillRule {
        /// Path index.
        path_index: usize,
        /// Rule type.
        rule_type: String,
    },
    /// `[relabel]`: the new string variables.
    Relabel(Vec<String>),
    /// `[typ]`: the new `.sco` / `.sli`.
    Typ(String),
    /// `[delete]`.
    Delete,
    /// Added elements (one or more consecutive blocks), parsed with the base layouts.
    Add(MapTile),
}

/// A parsed overlay tile.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TileOverlay {
    /// `[version]` of the overlay file.
    pub version: Option<u32>,
    /// The edits in file order.
    pub ops: Vec<OverlayOp>,
    /// Records that could not be read.
    pub warnings: Vec<String>,
}

/// What applying an overlay did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OverlayReport {
    /// Selections in the overlay.
    pub selections: usize,
    /// Selections that found their element.
    pub resolved: usize,
    /// Edits that could not be applied (unresolved selections, edits that do not fit the
    /// element, attachments whose carrier was deleted).
    pub warnings: Vec<String>,
}


impl TileOverlay {
    /// Parses an overlay tile from its bytes (UTF-16 or 8-bit, like base tiles).
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        Self::from_text(&decode_bytes(bytes))
    }

    /// Parses an overlay tile from decoded text. Blank lines are kept (a `[relabel]` value may be
    /// blank).
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
        let blocks = scan_overlay_blocks(&lines);
        let mut out = Self::default();
        let mut added = String::new();
        let mut selected = false;
        for (kw, line, body) in blocks {
            let values: Vec<&str> = body
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();
            let first = values.first().copied().unwrap_or_default();
            match kw.as_str() {
                "version" => {
                    if out.version.is_none() {
                        out.version = first.parse().ok();
                    }
                }
                "selobject" | "selspline" => {
                    flush_added_elements(&mut added, &mut out);
                    match first.parse::<u64>() {
                        Ok(id) if kw == "selobject" => out.ops.push(OverlayOp::SelectObject(id)),
                        Ok(id) => out.ops.push(OverlayOp::SelectSpline(id)),
                        Err(_) => {
                            out.warnings
                                .push(format!("line {line}: [{kw}] without an ID"));
                            out.ops.push(OverlayOp::SelectObject(0));
                        }
                    }
                    selected = true;
                }
                "rule" | "kill_rule" if selected => {
                    flush_added_elements(&mut added, &mut out);
                    parse_rule_op(&kw, line, &values, &mut out);
                }
                "relabel" | "typ" | "delete" | "kill_rule" if !selected => {
                    out.warnings
                        .push(format!("line {line}: [{kw}] without a selection: skipped"));
                }
                "relabel" => {
                    flush_added_elements(&mut added, &mut out);
                    parse_relabel_op(line, body, &mut out);
                }
                "typ" => {
                    flush_added_elements(&mut added, &mut out);
                    if first.is_empty() {
                        out.warnings
                            .push(format!("line {line}: [typ] without a file"));
                    } else {
                        out.ops.push(OverlayOp::Typ(first.to_string()));
                    }
                }
                "delete" => {
                    flush_added_elements(&mut added, &mut out);
                    out.ops.push(OverlayOp::Delete);
                }
                k if ADDED.contains(&k) => {
                    if k != "rule" {
                        selected = false;
                    }
                    added.push('[');
                    added.push_str(k);
                    added.push_str("]\n");
                    for l in body {
                        added.push_str(l);
                        added.push('\n');
                    }
                }
                _ => {}
            }
        }
        flush_added_elements(&mut added, &mut out);
        out
    }
}

mod apply;
mod blocks;

use blocks::{flush_added_elements, parse_rule_op, parse_relabel_op, scan_overlay_blocks, ADDED};

#[cfg(test)]
mod tests;
