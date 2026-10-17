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

use std::collections::{HashMap, HashSet};

use crate::formats::map::{MapTile, MapTrafficRule, RuleOwner};
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

/// Blocks an overlay adds (base layouts).
const ADDED: [&str; 10] = [
    "object",
    "spline",
    "spline_h",
    "attachobj",
    "splineattachement",
    "splineattachement_repeater",
    "varparent",
    "spline_terrain_align",
    "spline_terrain_align_2",
    "rule",
];

fn keyword(line: &str) -> Option<String> {
    let t = line.trim_end();
    (t.len() >= 2 && t.starts_with('[') && t.ends_with(']'))
        .then(|| t[1..t.len() - 1].to_ascii_lowercase())
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
        // Blocks: (keyword, line number, body lines up to the next keyword).
        let mut blocks: Vec<(String, usize, &[&str])> = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            if let Some(kw) = keyword(lines[i]) {
                let start = i + 1;
                let mut j = start;
                while j < lines.len() && keyword(lines[j]).is_none() {
                    j += 1;
                }
                blocks.push((kw, i + 1, &lines[start..j]));
                i = j;
            } else {
                i += 1;
            }
        }
        let mut out = Self::default();
        let mut added = String::new();
        let mut selected = false;
        let flush = |added: &mut String, out: &mut TileOverlay| {
            if added.is_empty() {
                return;
            }
            let version = out.version.unwrap_or(14);
            match MapTile::from_text(&format!("[version]\n{version}\n\n{added}")) {
                Ok(t) => out.ops.push(OverlayOp::Add(t)),
                Err(e) => out.warnings.push(format!("added records skipped: {e}")),
            }
            added.clear();
        };
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
                    flush(&mut added, &mut out);
                    match first.parse::<u64>() {
                        Ok(id) if kw == "selobject" => out.ops.push(OverlayOp::SelectObject(id)),
                        Ok(id) => out.ops.push(OverlayOp::SelectSpline(id)),
                        Err(_) => {
                            out.warnings
                                .push(format!("line {line}: [{kw}] without an ID"));
                            // Edits that follow have no element.
                            out.ops.push(OverlayOp::SelectObject(0));
                        }
                    }
                    selected = true;
                }
                "rule" | "kill_rule" if selected => {
                    flush(&mut added, &mut out);
                    let path_index = values.first().and_then(|v| v.parse::<usize>().ok());
                    let rule_type = values.get(1).map(|v| (*v).to_string());
                    match (path_index, rule_type) {
                        (Some(path_index), Some(rule_type)) if kw == "kill_rule" => {
                            out.ops.push(OverlayOp::KillRule {
                                path_index,
                                rule_type,
                            });
                        }
                        (Some(path_index), Some(rule_type)) => {
                            let value = values.get(2).and_then(|v| parse_float(v).ok());
                            let flag = values
                                .get(3)
                                .and_then(|v| v.parse::<u32>().ok())
                                .unwrap_or(0);
                            match value {
                                Some(value) => out.ops.push(OverlayOp::Rule(RuleEdit {
                                    path_index,
                                    rule_type,
                                    value,
                                    flag,
                                })),
                                None => out
                                    .warnings
                                    .push(format!("line {line}: [rule] without a value")),
                            }
                        }
                        _ => out.warnings.push(format!("line {line}: [{kw}] damaged")),
                    }
                }
                "relabel" | "typ" | "delete" | "kill_rule" if !selected => {
                    out.warnings
                        .push(format!("line {line}: [{kw}] without a selection: skipped"));
                }
                "relabel" => {
                    flush(&mut added, &mut out);
                    // `n`, then exactly `n` lines; blank lines are values.
                    let mut rest = body.iter().skip_while(|l| l.trim().is_empty());
                    let n = rest.next().and_then(|l| l.trim().parse::<usize>().ok());
                    match n {
                        Some(n) => {
                            let strings: Vec<String> =
                                rest.take(n).map(|l| l.trim().to_string()).collect();
                            if strings.len() < n {
                                out.warnings.push(format!(
                                    "line {line}: [relabel] has {} of {n} lines",
                                    strings.len()
                                ));
                            }
                            out.ops.push(OverlayOp::Relabel(strings));
                        }
                        None => out
                            .warnings
                            .push(format!("line {line}: [relabel] without a count")),
                    }
                }
                "typ" => {
                    flush(&mut added, &mut out);
                    if first.is_empty() {
                        out.warnings
                            .push(format!("line {line}: [typ] without a file"));
                    } else {
                        out.ops.push(OverlayOp::Typ(first.to_string()));
                    }
                }
                "delete" => {
                    flush(&mut added, &mut out);
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
                // Terrain/water markers, banners and unknown keywords: overlays never change them.
                _ => {}
            }
        }
        flush(&mut added, &mut out);
        out
    }
}

/// The element a selection found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Target {
    Object(usize),
    Attached(usize),
    SplineAttachment(usize),
    Spline(usize),
}

impl MapTile {
    /// Applies one overlay ([`TileOverlay`]) in place.
    ///
    /// Deleted elements are removed after the whole overlay (indices inside one overlay refer to
    /// the tile as it was before it plus the elements it added); rule owners, `[varparent]` links
    /// and attachment carriers are then re-numbered. Selections that find nothing skip their
    /// edits with a warning.
    pub fn apply_overlay(&mut self, overlay: &TileOverlay) -> OverlayReport {
        let mut report = OverlayReport::default();
        let mut dead: HashSet<Target> = HashSet::new();
        // `None`: no selection; `Some(None)`: a selection that found nothing.
        let mut sel: Option<Option<Target>> = None;
        for op in &overlay.ops {
            match op {
                OverlayOp::SelectObject(id) => {
                    report.selections += 1;
                    let alive = |t: Target| !dead.contains(&t);
                    let found = self
                        .objects
                        .iter()
                        .enumerate()
                        .filter(|(_, o)| o.id_code == *id)
                        .map(|(i, _)| Target::Object(i))
                        .find(|t| alive(*t))
                        .or_else(|| {
                            self.attached_objects
                                .iter()
                                .enumerate()
                                .filter(|(_, o)| o.id_code == *id)
                                .map(|(i, _)| Target::Attached(i))
                                .find(|t| alive(*t))
                        })
                        .or_else(|| {
                            self.spline_attachments
                                .iter()
                                .enumerate()
                                .filter(|(_, o)| o.id_code == *id)
                                .map(|(i, _)| Target::SplineAttachment(i))
                                .find(|t| alive(*t))
                        });
                    if found.is_some() {
                        report.resolved += 1;
                    } else {
                        report.warnings.push(format!(
                            "[selobject] {id}: no such element; its edits are skipped"
                        ));
                    }
                    sel = Some(found);
                }
                OverlayOp::SelectSpline(id) => {
                    report.selections += 1;
                    let found = self
                        .splines
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| s.id_code == *id)
                        .map(|(i, _)| Target::Spline(i))
                        .find(|t| !dead.contains(t));
                    if found.is_some() {
                        report.resolved += 1;
                    } else {
                        report.warnings.push(format!(
                            "[selspline] {id}: no such spline; its edits are skipped"
                        ));
                    }
                    sel = Some(found);
                }
                OverlayOp::Add(frag) => {
                    sel = None;
                    self.add_fragment(frag);
                    report.warnings.extend(frag.warnings.iter().cloned());
                }
                edit => {
                    let Some(Some(target)) = sel else {
                        continue;
                    };
                    if let Err(w) = self.edit(target, edit, &mut dead) {
                        report.warnings.push(w);
                    }
                }
            }
        }
        self.remove_dead(&dead, &mut report.warnings);
        report
    }

    fn rule_owner(&self, target: Target) -> Option<RuleOwner> {
        match target {
            Target::Object(index) => Some(RuleOwner::Object {
                index,
                id_code: self.objects[index].id_code,
            }),
            Target::Spline(index) => Some(RuleOwner::Spline {
                index,
                id_code: self.splines[index].id_code,
            }),
            _ => None,
        }
    }

    fn edit(
        &mut self,
        target: Target,
        op: &OverlayOp,
        dead: &mut HashSet<Target>,
    ) -> Result<(), String> {
        match op {
            OverlayOp::Rule(e) => {
                let owner = self.rule_owner(target).ok_or_else(|| {
                    format!("[rule] {} on an attached object: skipped", e.rule_type)
                })?;
                let existing = self.rules.iter_mut().find(|r| {
                    r.owner == Some(owner)
                        && r.path_index == e.path_index
                        && r.flag == e.flag
                        && r.rule_type.eq_ignore_ascii_case(&e.rule_type)
                });
                match existing {
                    Some(r) => r.value = e.value,
                    None => self.rules.push(MapTrafficRule {
                        owner: Some(owner),
                        path_index: e.path_index,
                        rule_type: e.rule_type.clone(),
                        value: e.value,
                        flag: e.flag,
                    }),
                }
            }
            OverlayOp::KillRule {
                path_index,
                rule_type,
            } => {
                let owner = self.rule_owner(target).ok_or_else(|| {
                    format!("[kill_rule] {rule_type} on an attached object: skipped")
                })?;
                self.rules.retain(|r| {
                    !(r.owner == Some(owner)
                        && r.path_index == *path_index
                        && r.rule_type.eq_ignore_ascii_case(rule_type))
                });
            }
            OverlayOp::Relabel(strings) => match target {
                Target::Object(i) => self.objects[i].string_vars.clone_from(strings),
                Target::Attached(i) => self.attached_objects[i].string_vars.clone_from(strings),
                Target::SplineAttachment(i) => {
                    return Err(format!(
                        "[relabel] of spline attachment {}: not applied",
                        self.spline_attachments[i].id_code
                    ))
                }
                Target::Spline(i) => {
                    return Err(format!(
                        "[relabel] of spline {}: not applied",
                        self.splines[i].id_code
                    ))
                }
            },
            OverlayOp::Typ(file) => match target {
                Target::Object(i) => self.objects[i].sco_file.clone_from(file),
                Target::Attached(i) => self.attached_objects[i].sco_file.clone_from(file),
                Target::SplineAttachment(i) => self.spline_attachments[i].sco_file.clone_from(file),
                Target::Spline(i) => self.splines[i].sli_file.clone_from(file),
            },
            OverlayOp::Delete => {
                dead.insert(target);
            }
            OverlayOp::SelectObject(_) | OverlayOp::SelectSpline(_) | OverlayOp::Add(_) => {}
        }
        Ok(())
    }

    /// Appends added elements; their rule owners and `[varparent]` links are shifted to the
    /// positions they take here (attachment carriers already count the whole tile).
    fn add_fragment(&mut self, frag: &MapTile) {
        let (o0, s0) = (self.objects.len(), self.splines.len());
        self.objects.extend(frag.objects.iter().cloned());
        self.splines.extend(frag.splines.iter().cloned());
        self.attached_objects
            .extend(frag.attached_objects.iter().cloned());
        self.spline_attachments
            .extend(frag.spline_attachments.iter().cloned());
        self.var_parents
            .extend(frag.var_parents.iter().map(|&(i, p)| (i + o0, p)));
        self.rules.extend(frag.rules.iter().map(|r| {
            let mut r = r.clone();
            r.owner = r.owner.map(|o| match o {
                RuleOwner::Object { index, id_code } => RuleOwner::Object {
                    index: index + o0,
                    id_code,
                },
                RuleOwner::Spline { index, id_code } => RuleOwner::Spline {
                    index: index + s0,
                    id_code,
                },
            });
            r
        }));
    }

    /// Removes deleted elements and re-numbers what refers to objects and splines by position.
    fn remove_dead(&mut self, dead: &HashSet<Target>, warnings: &mut Vec<String>) {
        if dead.is_empty() {
            return;
        }
        let remap = |len: usize, is_dead: &dyn Fn(usize) -> bool| -> HashMap<usize, usize> {
            let mut m = HashMap::new();
            let mut next = 0;
            for i in 0..len {
                if !is_dead(i) {
                    m.insert(i, next);
                    next += 1;
                }
            }
            m
        };
        let objects = remap(self.objects.len(), &|i| dead.contains(&Target::Object(i)));
        let splines = remap(self.splines.len(), &|i| dead.contains(&Target::Spline(i)));
        let n_splines = self.splines.len();
        keep_alive(&mut self.objects, |i| objects.contains_key(&i));
        keep_alive(&mut self.splines, |i| splines.contains_key(&i));
        keep_alive(&mut self.attached_objects, |i| {
            !dead.contains(&Target::Attached(i))
        });
        let mut i = 0;
        self.spline_attachments.retain_mut(|a| {
            let alive = !dead.contains(&Target::SplineAttachment(i));
            i += 1;
            if !alive {
                return false;
            }
            if a.spline_index < n_splines {
                match splines.get(&a.spline_index) {
                    Some(&n) => a.spline_index = n,
                    None => {
                        warnings.push(format!(
                            "attachment {} lost its carrier spline: removed",
                            a.id_code
                        ));
                        return false;
                    }
                }
            }
            true
        });
        self.var_parents
            .retain_mut(|(index, _)| match objects.get(index) {
                Some(&n) => {
                    *index = n;
                    true
                }
                None => false,
            });
        self.rules.retain_mut(|r| match r.owner {
            None => true,
            Some(RuleOwner::Object { index, id_code }) => objects.get(&index).is_some_and(|&n| {
                r.owner = Some(RuleOwner::Object { index: n, id_code });
                true
            }),
            Some(RuleOwner::Spline { index, id_code }) => splines.get(&index).is_some_and(|&n| {
                r.owner = Some(RuleOwner::Spline { index: n, id_code });
                true
            }),
        });
    }
}

fn keep_alive<T>(v: &mut Vec<T>, alive: impl Fn(usize) -> bool) {
    let mut i = 0;
    v.retain(|_| {
        let keep = alive(i);
        i += 1;
        keep
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(id: u64, sco: &str, strings: &[&str]) -> String {
        let mut s = format!(
            "[object]\n0\n{sco}\n{id}\n10\n20\n0\n90\n0\n0\n{}\n",
            strings.len()
        );
        for v in strings {
            s.push_str(v);
            s.push('\n');
        }
        s.push_str("Object Nr. 1\n\n");
        s
    }

    fn spline(id: u64, sli: &str) -> String {
        format!("[spline]\n0\n{sli}\n{id}\n0\n0\n0\n0\n0\n0\n50\n0\n0\n0\n0\n0\n0\n0\n\n")
    }

    fn rule(i: usize, t: &str, v: f32, f: u32) -> String {
        format!("[rule]\n{i}\n{t}\n{v}\n{f}\n\n")
    }

    fn base() -> MapTile {
        let text = format!(
            "[version]\n14\n\n[terrain]\n\n{}{}{}{}{}{}[attachObj]\n0\nline.sco\n300\n100\n0\n1\n0\n0\n0\n1\n13N\n\n[splineAttachement]\n0\nbollard.sco\n400\n1\n2\n0\n5\n0\n0\n0\n",
            object(100, "a.sco", &["x", "y"]),
            rule(0, "priority", 1.0, 0),
            spline(10, "road.sli"),
            rule(0, "trafficdensity", 1.0, 0),
            rule(1, "speedlimit", 50.0, 0),
            spline(11, "path.sli"),
        );
        MapTile::from_text(&text).unwrap()
    }

    fn apply(tile: &mut MapTile, overlay: &str) -> OverlayReport {
        tile.apply_overlay(&TileOverlay::from_text(overlay))
    }

    #[test]
    fn rule_upserts_by_owner_path_type_and_flag() {
        let mut t = base();
        let r = apply(
            &mut t,
            &format!(
                "[selspline]\n10\n\n{}{}",
                rule(0, "TrafficDensity", 4.0, 0),
                rule(0, "trafficdensity", 2.0, 3)
            ),
        );
        assert_eq!((r.selections, r.resolved), (1, 1));
        let on10: Vec<_> = t
            .rules
            .iter()
            .filter(|r| matches!(r.owner, Some(RuleOwner::Spline { id_code: 10, .. })))
            .collect();
        assert_eq!(on10.len(), 3, "updated one, added one with another flag");
        assert_eq!(on10[0].value, 4.0);
        assert_eq!((on10[2].value, on10[2].flag), (2.0, 3));
    }

    #[test]
    fn kill_rule_removes_by_path_and_type() {
        let mut t = base();
        apply(
            &mut t,
            "[selspline]\n10\n[kill_rule]\n1\nspeedlimit\n0\n7\n",
        );
        assert!(!t.rules.iter().any(|r| r.rule_type == "speedlimit"));
        assert_eq!(t.rules.len(), 2);
    }

    /// `[relabel]` takes exactly `n` lines, blank ones included, and replaces all strings.
    #[test]
    fn relabel_keeps_blank_values() {
        let mut t = base();
        apply(&mut t, "[selobject]\n100\n[relabel]\n3\nfirst\n\nthird\nObject Nr. 5\n\n[selobject]\n300\n[relabel]\n1\n\n");
        assert_eq!(t.objects[0].string_vars, vec!["first", "", "third"]);
        assert_eq!(t.attached_objects[0].string_vars, vec![""]);
    }

    #[test]
    fn typ_replaces_sco_and_sli() {
        let mut t = base();
        apply(&mut t, "[selobject]\n300\n[typ]\nnew_line.sco\n[selobject]\n400\n[typ]\nnew_bollard.sco\n[selspline]\n11\n[typ]\nnew.sli\n");
        assert_eq!(t.attached_objects[0].sco_file, "new_line.sco");
        assert_eq!(t.spline_attachments[0].sco_file, "new_bollard.sco");
        assert_eq!(t.splines[1].sli_file, "new.sli");
    }

    /// Deleting the first spline moves the second down: rule owners and attachment carriers are
    /// re-numbered, the deleted spline's rules go.
    #[test]
    fn delete_renumbers_owners_and_carriers() {
        let mut t = base();
        let r = apply(&mut t, "[selspline]\n10\n[delete]\n");
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(t.splines.len(), 1);
        assert_eq!(
            t.spline_attachments[0].spline_index, 0,
            "carrier 11 is now first"
        );
        assert_eq!(t.rules.len(), 1, "only the object's rule is left");
        // Deleting the carrier removes its attachment.
        let mut t = base();
        let r = apply(&mut t, "[selspline]\n11\n[delete]\n");
        assert!(t.spline_attachments.is_empty());
        assert_eq!(r.warnings.len(), 1);
        // Deleting an object.
        let mut t = base();
        apply(
            &mut t,
            "[selobject]\n100\n[delete]\n[selobject]\n300\n[delete]\n",
        );
        assert!(t.objects.is_empty() && t.attached_objects.is_empty());
        assert!(!t
            .rules
            .iter()
            .any(|r| matches!(r.owner, Some(RuleOwner::Object { .. }))));
    }

    /// The selection persists over several edits and is cleared by an added element: the rule
    /// after it belongs to the new spline. Added attachments count carriers over the whole tile.
    #[test]
    fn selection_persists_and_added_elements_take_their_rules() {
        let mut t = base();
        let overlay = format!(
            "[version]\n14\n[selobject]\n100\n{}[relabel]\n1\nz\n{}{}[splineAttachement]\n0\nsign.sco\n500\n2\n1\n0\n3\n0\n0\n0\n{}[selspline]\n12\n{}",
            rule(0, "priority", 2.0, 0),
            spline(12, "new.sli"),
            rule(0, "speedlimit", 30.0, 0),
            object(101, "b.sco", &[]),
            rule(1, "trafficdensity", 0.0, 0),
        );
        let r = apply(&mut t, &overlay);
        assert_eq!(
            (r.selections, r.resolved),
            (2, 2),
            "the second selection finds the added spline"
        );
        assert_eq!(t.objects[0].string_vars, vec!["z"]);
        assert_eq!(t.splines.len(), 3);
        assert_eq!(t.objects.len(), 2);
        let on12: Vec<_> = t
            .rules
            .iter()
            .filter(|r| {
                r.owner
                    == Some(RuleOwner::Spline {
                        index: 2,
                        id_code: 12,
                    })
            })
            .map(|r| r.rule_type.as_str())
            .collect();
        assert_eq!(on12, vec!["speedlimit", "trafficdensity"]);
        assert_eq!(t.spline_attachments[1].spline_index, 2);
        assert!(t
            .rules
            .iter()
            .any(|r| r.rule_type == "priority" && r.value == 2.0));
    }

    #[test]
    fn unresolved_selections_skip_their_edits() {
        let mut t = base();
        let before = t.clone();
        let r = apply(&mut t, "[selobject]\n999\n[delete]\n[relabel]\n1\nq\n");
        assert_eq!((r.selections, r.resolved), (1, 0));
        assert_eq!(r.warnings.len(), 1);
        assert_eq!(t, before);
    }

    /// Overlays never touch terrain or water markers.
    #[test]
    fn terrain_markers_are_ignored() {
        let mut t = MapTile::default();
        apply(&mut t, "[version]\n14\n[terrain]\n[water]\n");
        assert!(!t.has_terrain && !t.has_water);
    }
}
