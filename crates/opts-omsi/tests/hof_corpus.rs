//! Corpus test: every `.hof` file below `Vehicles/` of an installation parses (M-0099), and what
//! the parser reads is consistent: terminus codes of the routes exist in the terminus table, the
//! stops of a route exist in the bus-stop table. Skipped unless `OMSI_PATH` points at an
//! installation. Identical copies (the same depot is shipped into dozens of vehicle folders) are
//! parsed once.

use std::collections::{BTreeMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use opts_omsi::formats::hof::HofFile;

fn hofs(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() && depth < 3 {
            hofs(&p, out, depth + 1);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("hof")) {
            out.push(p);
        }
    }
}

#[test]
fn every_hof_of_the_installation_parses_and_is_consistent() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set: skipped");
        return;
    };
    let mut files = Vec::new();
    hofs(&root.join("Vehicles"), &mut files, 0);
    files.sort();
    let mut seen = HashSet::new();
    let (mut parsed, mut copies) = (0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();
    let mut dangling_terminus = 0usize;
    let mut dangling_stop = 0usize;
    let mut routes = 0usize;
    let mut stop_refs = 0usize;
    let mut worst: BTreeMap<usize, String> = BTreeMap::new();
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            failures.push(format!("{}: unreadable", f.display()));
            continue;
        };
        let mut h = DefaultHasher::new();
        bytes.hash(&mut h);
        if !seen.insert(h.finish()) {
            copies += 1;
            continue;
        }
        match HofFile::parse_bytes(&bytes) {
            Ok(hof) => {
                parsed += 1;
                let mut local = 0usize;
                for r in &hof.routes {
                    routes += 1;
                    if hof.find_terminus_by_code(r.terminus_code).is_none() {
                        dangling_terminus += 1;
                        local += 1;
                    }
                    for s in &r.busstops {
                        stop_refs += 1;
                        if hof.find_busstop_by_ident(s).is_none() {
                            dangling_stop += 1;
                            local += 1;
                        }
                    }
                }
                if local > 0 {
                    worst.insert(local, f.display().to_string());
                }
            }
            Err(e) => failures.push(format!("{}: {e}", f.display())),
        }
    }
    eprintln!(
        "{} hof files, {} distinct parsed, {} identical copies, {} routes, {} routes with an unknown terminus, {} of {} stop idents unknown; worst file: {:?}",
        files.len(),
        parsed,
        copies,
        routes,
        dangling_terminus,
        dangling_stop,
        stop_refs,
        worst.iter().next_back()
    );
    assert!(
        failures.is_empty(),
        "{} failed: {:?}",
        failures.len(),
        &failures[..failures.len().min(10)]
    );
    assert!(parsed > 100, "the corpus is not there");
    // What stays unresolved is content (platform suffixes such as `Teltow Bahnhof*N37`, stops that
    // only exist in a vehicle's own list); `docs/src/formats/text/vehicles/hof.md` records it.
    assert!(
        dangling_stop * 50 < stop_refs && dangling_terminus * 50 < routes,
        "{dangling_stop}/{stop_refs} stops, {dangling_terminus}/{routes} termini unresolved"
    );
}
