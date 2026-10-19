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

#[test]
fn every_rule_has_an_owner_and_a_valid_path_index() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let mut path_counts: HashMap<String, Option<usize>> = HashMap::new();
    // Number of addressable paths: a spline's own; an object's own plus those of the spline
    // profiles it embeds with `[splinehelper]` (rule indices continue into them).
    fn profile_paths(root: &Path, file: &str) -> Option<usize> {
        let bytes = std::fs::read(find_ci(root, file)?).ok()?;
        SplineProfile::parse(&bytes).ok().map(|p| p.paths.len())
    }
    let mut count = |file: &str, spline: bool| -> Option<usize> {
        *path_counts
            .entry(file.to_ascii_lowercase())
            .or_insert_with(|| {
                if spline {
                    return profile_paths(&root, file);
                }
                let bytes = std::fs::read(find_ci(&root, file)?).ok()?;
                let o = SceneryObject::parse(&bytes).ok()?;
                let helpers: usize = o
                    .spline_helpers
                    .iter()
                    .map(|h| profile_paths(&root, &h.profile_file).unwrap_or(0))
                    .sum();
                Some(o.paths.len() + helpers)
            })
    };
    let (mut rules, mut no_owner, mut in_range, mut out_of_range, mut unknown_file) =
        (0usize, 0, 0, 0, 0);
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut owners: BTreeMap<&str, usize> = BTreeMap::new();
    let mut values: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut tiles = 0usize;
    let mut examples: Vec<String> = Vec::new();
    for e in std::fs::read_dir(root.join("maps")).unwrap().flatten() {
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
            tiles += 1;
            for r in &tile.rules {
                rules += 1;
                let kind = r
                    .kind()
                    .map_or_else(|| format!("? {}", r.rule_type), |k| format!("{k:?}"));
                *by_kind.entry(kind.clone()).or_default() += 1;
                *values
                    .entry(kind)
                    .or_default()
                    .entry(format!("{} / {}", r.value, r.flag))
                    .or_default() += 1;
                let owner_file;
                let n = match r.owner {
                    None => {
                        no_owner += 1;
                        continue;
                    }
                    Some(RuleOwner::Spline { index, id_code }) => {
                        assert_eq!(tile.splines[index].id_code, id_code);
                        *owners.entry("spline").or_default() += 1;
                        owner_file = tile.splines[index].sli_file.clone();
                        count(&owner_file, true)
                    }
                    Some(RuleOwner::Object { index, id_code }) => {
                        assert_eq!(tile.objects[index].id_code, id_code);
                        *owners.entry("object").or_default() += 1;
                        owner_file = tile.objects[index].sco_file.clone();
                        count(&owner_file, false)
                    }
                };
                match n {
                    None => unknown_file += 1,
                    Some(n) if r.path_index < n => in_range += 1,
                    Some(n) => {
                        out_of_range += 1;
                        if examples.len() < 8 {
                            examples.push(format!(
                                "{owner_file}: index {} of {n} paths ({:?})",
                                r.path_index,
                                r.kind()
                            ));
                        }
                    }
                }
            }
        }
    }
    eprintln!("{tiles} tiles, {rules} rules: {no_owner} without owner, owners {owners:?}");
    eprintln!("path index in range: {in_range}, out of range: {out_of_range}, owner file unknown: {unknown_file}");
    eprintln!("out-of-range examples: {examples:#?}");
    eprintln!("by kind: {by_kind:?}");
    for (k, v) in &values {
        let mut top: Vec<_> = v.iter().collect();
        top.sort_by(|a, b| b.1.cmp(a.1));
        eprintln!("  {k}: {:?}", &top[..top.len().min(6)]);
    }
    assert_eq!(no_owner, 0, "every rule should follow a spline or object");
}
