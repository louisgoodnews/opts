//! Where do a vehicle's setvars come from? Walks every `.bus` below `<OMSI root>/Vehicles`,
//! follows its `[model]` to the `[CTC]` schemes, reads the `.cti` files of every scheme folder
//! (not only the one named `Colorscheme` that the wizard reads today) and joins the `[setvar]`
//! names with the variables the vehicle declares in its `[varnamelist]` files.
//!
//! M-0078 census, `docs/src/formats/text/vehicles/setvars.md`. Usage: `-- <OMSI root>`.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use opts_core::formats::cti::CtiFile;
use opts_core::formats::varlist::VarList;
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

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

fn buses(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            buses(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bus")) {
            out.push(p);
        }
    }
}

/// One vehicle: what its schemes, repaints and varlists say.
#[derive(Default)]
struct Vehicle {
    /// `[CTC]` scheme name to `(.cti files, setvar lines)`.
    schemes: BTreeMap<String, (usize, usize)>,
    /// Setvar variable (lower case) to the values repaints give it.
    setvars: BTreeMap<String, BTreeSet<String>>,
    /// Setvar variable (lower case) to the scheme names it appears in.
    scheme_of: BTreeMap<String, BTreeSet<String>>,
    /// Declared float variables (lower case) and the names of the lists that declare them.
    declared: BTreeMap<String, BTreeSet<String>>,
    /// Names of the declaring lists.
    lists: BTreeSet<String>,
}

fn read_vehicle(root: &Path, bus: &Path) -> Option<Vehicle> {
    let dir = bus.parent()?.to_path_buf();
    let manifest = BusManifest::parse_bytes(&std::fs::read(bus).ok()?).ok()?;
    let mut v = Vehicle::default();
    for rel in &manifest.varnamelists {
        let Some(path) = find_ci(&dir, rel).or_else(|| find_ci(root, rel)) else {
            continue;
        };
        let list = path.file_name()?.to_string_lossy().to_lowercase();
        v.lists.insert(list.clone());
        for name in VarList::parse(&std::fs::read(&path).ok()?).names {
            v.declared
                .entry(name.to_lowercase())
                .or_default()
                .insert(list.clone());
        }
    }
    let cfg = manifest
        .model_cfg
        .as_deref()
        .and_then(|c| find_ci(&dir, c))?;
    let model = ModelConfig::parse_bytes(&std::fs::read(cfg).ok()?).ok()?;
    let bases: Vec<PathBuf> = std::iter::once(dir.clone())
        .chain(dir.ancestors().skip(1).take(2).map(Path::to_path_buf))
        .chain(std::iter::once(root.to_path_buf()))
        .collect();
    for scheme in &model.ctc_schemes {
        let Some(folder) = bases
            .iter()
            .find_map(|b| find_ci(b, &scheme.texture_path))
            .filter(|p| p.is_dir())
        else {
            continue;
        };
        let entry = v.schemes.entry(scheme.name.clone()).or_default();
        for f in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            let p = f.path();
            if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("cti")) {
                continue;
            }
            let Ok(bytes) = std::fs::read(&p) else {
                continue;
            };
            let cti = CtiFile::parse(&bytes);
            entry.0 += 1;
            entry.1 += cti.setvars.len();
            for sv in cti.setvars {
                let name = sv.variable.to_lowercase();
                v.setvars.entry(name.clone()).or_default().insert(sv.value);
                v.scheme_of
                    .entry(name)
                    .or_default()
                    .insert(scheme.name.clone());
            }
        }
    }
    Some(v)
}

fn median(mut v: Vec<usize>) -> usize {
    v.sort_unstable();
    v.get(v.len() / 2).copied().unwrap_or(0)
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("OMSI root"));
    let mut files = Vec::new();
    buses(&root.join("Vehicles"), &mut files);
    files.sort();
    let (mut parsed, mut skipped) = (0usize, 0usize);
    let mut vehicles: Vec<(String, Vehicle)> = Vec::new();
    for bus in &files {
        match read_vehicle(&root, bus) {
            Some(v) => {
                parsed += 1;
                vehicles.push((bus.display().to_string(), v));
            }
            None => skipped += 1,
        }
    }
    println!(
        ".bus files {}: read {parsed}, unreadable or without a model {skipped}",
        files.len()
    );
    let with_cti = vehicles
        .iter()
        .filter(|(_, v)| v.schemes.values().any(|s| s.0 > 0))
        .count();
    let with_sv = vehicles
        .iter()
        .filter(|(_, v)| !v.setvars.is_empty())
        .count();
    println!("vehicles with .cti files {with_cti}, with at least one [setvar] {with_sv}");

    // 1. Schemes: which names carry the .cti files and the setvar lines.
    let mut by_scheme: BTreeMap<String, (usize, usize, usize, usize)> = BTreeMap::new();
    for (_, v) in &vehicles {
        for (name, (cti, sv)) in &v.schemes {
            let e = by_scheme.entry(name.to_lowercase()).or_default();
            e.0 += 1;
            e.1 += cti;
            e.2 += sv;
            e.3 += usize::from(*sv > 0);
        }
    }
    println!(
        "\n## CTC schemes (name: vehicles, .cti files, [setvar] lines, vehicles with setvars)"
    );
    let mut rows: Vec<_> = by_scheme.iter().collect();
    rows.sort_by_key(|(_, e)| std::cmp::Reverse(e.2));
    let total_lines: usize = by_scheme.values().map(|e| e.2).sum();
    for (name, e) in rows.iter().take(20) {
        println!("{name:28} {:5} {:6} {:7} {:5}", e.0, e.1, e.2, e.3);
    }
    let colour: usize = by_scheme.get("colorscheme").map_or(0, |e| e.2);
    println!(
        "[setvar] lines in all schemes {total_lines}; in `Colorscheme` (what the wizard reads) {colour}; in other schemes {}",
        total_lines - colour
    );
    let only_other = vehicles
        .iter()
        .filter(|(_, v)| {
            v.scheme_of
                .values()
                .any(|s| !s.iter().any(|n| n.eq_ignore_ascii_case("Colorscheme")))
        })
        .count();
    println!("vehicles with a setvar that appears only outside `Colorscheme`: {only_other}");

    // 2. Names and how many vehicles use them.
    let mut names: BTreeMap<&str, (usize, BTreeSet<&str>)> = BTreeMap::new();
    for (_, v) in &vehicles {
        for (name, values) in &v.setvars {
            let e = names.entry(name).or_default();
            e.0 += 1;
            e.1.extend(values.iter().map(String::as_str));
        }
    }
    // Optional machine-readable dump for cross-checks: SETVAR_JSON=<file>.
    if let Some(path) = std::env::var_os("SETVAR_JSON") {
        let mut json = String::from("{");
        for (i, (name, (count, values))) in names.iter().enumerate() {
            let vals: Vec<String> = values.iter().map(|v| format!("{v:?}")).collect();
            json.push_str(&format!(
                "{}{name:?}:{{\"vehicles\":{count},\"values\":[{}]}}",
                if i == 0 { "" } else { "," },
                vals.join(",")
            ));
        }
        json.push('}');
        std::fs::write(path, json).expect("write json");
    }
    // Per vehicle dump for per-family cross-checks: SETVAR_VEHICLES_JSON=<file>.
    if let Some(path) = std::env::var_os("SETVAR_VEHICLES_JSON") {
        let mut json = String::from("{");
        let mut first = true;
        for (bus, v) in vehicles.iter().filter(|(_, v)| !v.setvars.is_empty()) {
            if !first {
                json.push(',');
            }
            first = false;
            json.push_str(&format!("{bus:?}:{{"));
            let items: Vec<String> = v
                .setvars
                .iter()
                .map(|(n, vals)| {
                    let vs: Vec<String> = vals.iter().map(|x| format!("{x:?}")).collect();
                    format!("{n:?}:[{}]", vs.join(","))
                })
                .collect();
            json.push_str(&items.join(","));
            json.push('}');
        }
        json.push('}');
        std::fs::write(path, json).expect("write json");
    }
    println!("\n## Setvar names: {} distinct", names.len());
    let per_vehicle: Vec<usize> = vehicles
        .iter()
        .filter(|(_, v)| !v.setvars.is_empty())
        .map(|(_, v)| v.setvars.len())
        .collect();
    println!(
        "per vehicle (with setvars): median {}, max {}",
        median(per_vehicle.clone()),
        per_vehicle.iter().max().copied().unwrap_or(0)
    );
    let used_once = names.values().filter(|e| e.0 == 1).count();
    println!("names used by exactly one vehicle {used_once}");
    let mut top: Vec<_> = names.iter().collect();
    top.sort_by_key(|(n, e)| (std::cmp::Reverse(e.0), **n));
    println!("top names by vehicles (name: vehicles, distinct values):");
    for (n, e) in top.iter().take(25) {
        let mut vals: Vec<&&str> = e.1.iter().collect();
        vals.sort_by_key(|s| s.parse::<i64>().unwrap_or(i64::MAX));
        let shown: Vec<String> = vals.iter().take(6).map(|s| (***s).to_string()).collect();
        println!("  {n:32} {:4} {:3}  {}", e.0, e.1.len(), shown.join(","));
    }
    let (mut int_only, mut binary, mut other) = (0, 0, 0);
    for (_, vals) in names.values() {
        if vals.iter().all(|v| v.parse::<i64>().is_ok()) {
            if vals.iter().all(|v| *v == "0" || *v == "1") {
                binary += 1;
            } else {
                int_only += 1;
            }
        } else {
            other += 1;
        }
    }
    println!("value shapes by name: only 0/1 {binary}, other integers {int_only}, not all integers {other}");
    // Shape of the observed value set of each name (a lower bound of what the option accepts).
    let mut shapes: BTreeMap<&str, usize> = BTreeMap::new();
    let mut not_int: Vec<(&str, Vec<&str>)> = Vec::new();
    let mut widest: Vec<(&str, i64)> = Vec::new();
    for (name, (_, vals)) in &names {
        let ints: Option<BTreeSet<i64>> = vals.iter().map(|v| v.parse().ok()).collect();
        let shape = match ints {
            None => {
                not_int.push((name, vals.iter().copied().collect()));
                "not integers"
            }
            Some(set) => {
                let (lo, hi) = (
                    *set.iter().next().unwrap_or(&0),
                    *set.iter().next_back().unwrap_or(&0),
                );
                widest.push((name, hi));
                let contiguous = (hi - lo + 1) as usize == set.len();
                match (lo, hi, contiguous, set.len()) {
                    (_, _, _, 1) if lo == 1 => "single value 1 (switch on)",
                    (_, _, _, 1) if lo == 0 => "single value 0",
                    (_, _, _, 1) => "single other value",
                    (0, 1, _, _) => "0 and 1",
                    (1, _, true, _) => "1..N contiguous",
                    (0, _, true, _) => "0..N contiguous",
                    (_, _, true, _) => "contiguous from another start",
                    _ => "with gaps",
                }
            }
        };
        *shapes.entry(shape).or_default() += 1;
    }
    println!("shapes of the observed value sets (names):");
    for (shape, n) in &shapes {
        println!("  {shape:32} {n}");
    }
    widest.sort_by_key(|(n, hi)| (std::cmp::Reverse(*hi), *n));
    println!(
        "largest observed values: {:?}",
        &widest[..widest.len().min(8)]
    );
    println!("names with non-integer values: {not_int:?}");

    // 3. Declared or not.
    let (mut declared, mut undeclared) = (0usize, 0usize);
    let mut undeclared_names: BTreeMap<&str, usize> = BTreeMap::new();
    let mut in_visible_list = 0usize;
    for (_, v) in &vehicles {
        for name in v.setvars.keys() {
            match v.declared.get(name) {
                Some(lists) => {
                    declared += 1;
                    if lists.iter().any(|l| l.contains("visib")) {
                        in_visible_list += 1;
                    }
                }
                None => {
                    undeclared += 1;
                    *undeclared_names.entry(name).or_default() += 1;
                }
            }
        }
    }
    // Which lists declare the setvars (file name, pairs, vehicles).
    let mut by_list: BTreeMap<String, (usize, BTreeSet<usize>)> = BTreeMap::new();
    for (i, (_, v)) in vehicles.iter().enumerate() {
        for name in v.setvars.keys() {
            for list in v.declared.get(name).into_iter().flatten() {
                let e = by_list.entry(list.clone()).or_default();
                e.0 += 1;
                e.1.insert(i);
            }
        }
    }
    println!("\n## Lists that declare setvar names (file name: setvar pairs, vehicles)");
    let mut lists: Vec<_> = by_list.iter().collect();
    lists.sort_by_key(|(n, e)| (std::cmp::Reverse(e.0), (*n).clone()));
    for (n, e) in lists.iter().take(14) {
        println!("  {n:36} {:6} {:5}", e.0, e.1.len());
    }
    // A list whose file name says `setvar`: does it declare more than the repaints set?
    let (mut with_list, mut list_names, mut list_unset, mut set_unlisted) =
        (0usize, 0usize, 0usize, 0usize);
    let mut examples: Vec<(String, usize, usize, usize)> = Vec::new();
    for (path, v) in &vehicles {
        let declared_by_list: BTreeSet<&String> = v
            .declared
            .iter()
            .filter(|(_, ls)| ls.iter().any(|l| l.contains("setvar")))
            .map(|(n, _)| n)
            .collect();
        if declared_by_list.is_empty() {
            continue;
        }
        with_list += 1;
        let set: BTreeSet<&String> = v.setvars.keys().collect();
        let unset = declared_by_list.difference(&set).count();
        let unlisted = set.difference(&declared_by_list).count();
        list_names += declared_by_list.len();
        list_unset += unset;
        set_unlisted += unlisted;
        if examples.len() < 5 && unset > 0 {
            examples.push((
                path.rsplit('/').next().unwrap_or(path).to_string(),
                declared_by_list.len(),
                set.len(),
                unset,
            ));
        }
    }
    println!(
        "\n## Lists named *setvar*: {with_list} vehicles declare {list_names} names; {list_unset} of them no repaint of that vehicle sets; {set_unlisted} setvar names the repaints set are not in such a list"
    );
    for (bus, declared, set, unset) in &examples {
        println!("  e.g. {bus}: list {declared} names, repaints set {set}, declared-but-never-set {unset}");
    }
    println!(
        "\n## Join with the varlists (vehicle, setvar) pairs: declared {declared}, not declared {undeclared}; declared in a list whose file name contains `visib`: {in_visible_list}"
    );
    let with_visible = vehicles
        .iter()
        .filter(|(_, v)| v.lists.iter().any(|l| l.contains("visib")))
        .count();
    println!("vehicles that have a varlist with `visib` in its file name: {with_visible}");
    let mut und: Vec<_> = undeclared_names.iter().collect();
    und.sort_by_key(|(n, c)| (std::cmp::Reverse(**c), **n));
    println!("most frequent setvar names a vehicle does not declare (name: vehicles):");
    for (n, c) in und.iter().take(15) {
        println!("  {n:32} {c}");
    }
}
