//! The tallies of the tile rule corpus test: kinds, values, owners and path-index ranges.

use std::collections::BTreeMap;

use opts_omsi::formats::map::{MapTile, RuleOwner};

use super::paths::PathCounts;
/// What the test counts over the corpus.
#[derive(Default)]
pub(super) struct Stats {
    pub(super) tiles: usize,
    pub(super) rules: usize,
    pub(super) no_owner: usize,
    pub(super) in_range: usize,
    pub(super) out_of_range: usize,
    pub(super) unknown_file: usize,
    pub(super) by_kind: BTreeMap<String, usize>,
    pub(super) owners: BTreeMap<&'static str, usize>,
    pub(super) values: BTreeMap<String, BTreeMap<String, usize>>,
    pub(super) examples: Vec<String>,
}

impl Stats {
    /// One rule: its kind and value, its owner and whether its path index is in range.
    pub(super) fn rule(
        &mut self,
        tile: &MapTile,
        r: &opts_omsi::formats::map::MapTrafficRule,
        counts: &mut PathCounts,
    ) {
        self.rules += 1;
        let kind = r
            .kind()
            .map_or_else(|| format!("? {}", r.rule_type), |k| format!("{k:?}"));
        *self.by_kind.entry(kind.clone()).or_default() += 1;
        *self
            .values
            .entry(kind)
            .or_default()
            .entry(format!("{} / {}", r.value, r.flag))
            .or_default() += 1;
        let (owner_file, n) = match r.owner {
            None => {
                self.no_owner += 1;
                return;
            }
            Some(RuleOwner::Spline { index, id_code }) => {
                assert_eq!(tile.splines[index].id_code, id_code);
                *self.owners.entry("spline").or_default() += 1;
                let file = tile.splines[index].sli_file.clone();
                let n = counts.count(&file, true);
                (file, n)
            }
            Some(RuleOwner::Object { index, id_code }) => {
                assert_eq!(tile.objects[index].id_code, id_code);
                *self.owners.entry("object").or_default() += 1;
                let file = tile.objects[index].sco_file.clone();
                let n = counts.count(&file, false);
                (file, n)
            }
        };
        match n {
            None => self.unknown_file += 1,
            Some(n) if r.path_index < n => self.in_range += 1,
            Some(n) => {
                self.out_of_range += 1;
                if self.examples.len() < 8 {
                    self.examples.push(format!(
                        "{owner_file}: index {} of {n} paths ({:?})",
                        r.path_index,
                        r.kind()
                    ));
                }
            }
        }
    }

    pub(super) fn report(&self) {
        let (tiles, rules, no_owner) = (self.tiles, self.rules, self.no_owner);
        let (owners, examples, by_kind) = (&self.owners, &self.examples, &self.by_kind);
        let (in_range, out_of_range, unknown_file) =
            (self.in_range, self.out_of_range, self.unknown_file);
        eprintln!("{tiles} tiles, {rules} rules: {no_owner} without owner, owners {owners:?}");
        eprintln!("path index in range: {in_range}, out of range: {out_of_range}, owner file unknown: {unknown_file}");
        eprintln!("out-of-range examples: {examples:#?}");
        eprintln!("by kind: {by_kind:?}");
        for (k, v) in &self.values {
            let mut top: Vec<_> = v.iter().collect();
            top.sort_by(|a, b| b.1.cmp(a.1));
            eprintln!("  {k}: {:?}", &top[..top.len().min(6)]);
        }
    }
}
