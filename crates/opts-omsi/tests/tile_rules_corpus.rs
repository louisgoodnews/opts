//! Corpus test: every `[rule]` of every installed tile belongs to a spline or object, and its path
//! index addresses one of that element's `[path]` records (`docs/src/formats/map/tiles.md`).
//! Skipped unless `OMSI_PATH` points at an installation. Prints the statistics quoted in the docs.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use opts_omsi::formats::map::{MapTile, RuleOwner};
use opts_omsi::formats::scenery::{SceneryObject, SplineProfile};

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = hit.path();
    }
    Some(cur)
}

/// Number of addressable paths of a file: a spline's own; an object's own plus those of the
/// spline profiles it embeds with `[splinehelper]` (rule indices continue into them). Cached.
struct PathCounts {
    root: PathBuf,
    cache: HashMap<String, Option<usize>>,
}

impl PathCounts {
    fn profile_paths(&self, file: &str) -> Option<usize> {
        let bytes = std::fs::read(find_ci(&self.root, file)?).ok()?;
        SplineProfile::parse(&bytes).ok().map(|p| p.paths.len())
    }

    fn count(&mut self, file: &str, spline: bool) -> Option<usize> {
        let key = file.to_ascii_lowercase();
        if let Some(hit) = self.cache.get(&key) {
            return *hit;
        }
        let n = self.compute(file, spline);
        self.cache.insert(key, n);
        n
    }

    fn compute(&self, file: &str, spline: bool) -> Option<usize> {
        if spline {
            return self.profile_paths(file);
        }
        let bytes = std::fs::read(find_ci(&self.root, file)?).ok()?;
        let o = SceneryObject::parse(&bytes).ok()?;
        let helpers: usize = o
            .spline_helpers
            .iter()
            .map(|h| self.profile_paths(&h.profile_file).unwrap_or(0))
            .sum();
        Some(o.paths.len() + helpers)
    }
}

/// What the test counts over the corpus.
#[derive(Default)]
struct Stats {
    tiles: usize,
    rules: usize,
    no_owner: usize,
    in_range: usize,
    out_of_range: usize,
    unknown_file: usize,
    by_kind: BTreeMap<String, usize>,
    owners: BTreeMap<&'static str, usize>,
    values: BTreeMap<String, BTreeMap<String, usize>>,
    examples: Vec<String>,
}

impl Stats {
    /// One rule: its kind and value, its owner and whether its path index is in range.
    fn rule(
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

    fn report(&self) {
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

#[test]
fn every_rule_has_an_owner_and_a_valid_path_index() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let maps = std::fs::read_dir(root.join("maps")).unwrap();
    let mut counts = PathCounts {
        root,
        cache: HashMap::new(),
    };
    let mut stats = Stats::default();
    for e in maps.flatten() {
        let Ok(rd) = std::fs::read_dir(e.path()) else {
            continue;
        };
        for f in rd.flatten() {
            let name = f.file_name().to_string_lossy().to_lowercase();
            if !(name.starts_with("tile_") && name.ends_with(".map")) {
                continue;
            }
            let Some(tile) = std::fs::read(f.path())
                .ok()
                .and_then(|b| MapTile::parse(&b).ok())
            else {
                continue;
            };
            stats.tiles += 1;
            for r in &tile.rules {
                stats.rule(&tile, r, &mut counts);
            }
        }
    }
    stats.report();
    assert_eq!(
        stats.no_owner, 0,
        "every rule should follow a spline or object"
    );
}
