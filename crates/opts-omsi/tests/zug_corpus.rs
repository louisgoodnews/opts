//! Corpus test: the `.zug` files of a real OMSI installation parse (script text saved as `.zug`
//! is refused). Skipped unless `OMSI_PATH` points at an installation.

use std::path::{Path, PathBuf};

use opts_omsi::formats::trains::{TrainFormation, VehicleRef, ZugError};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("zug")) {
                out.push(p);
            }
        }
    }
}

#[test]
fn every_consist_in_trains_parses_and_stray_scripts_are_refused() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let mut files = Vec::new();
    walk(&root.join("Trains"), &mut files);
    assert!(
        files.len() >= 100,
        "only {} .zug under Trains/",
        files.len()
    );
    let (mut parsed, mut refused, mut groups, mut longest, mut ovh, mut bus) =
        (0, Vec::new(), 0, 0, 0, 0);
    for f in &files {
        let rel = f
            .strip_prefix(&root)
            .unwrap_or(f)
            .to_string_lossy()
            .into_owned();
        let name = f
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        match TrainFormation::from_zug(&std::fs::read(f).expect("read"), &name, &rel) {
            Ok((t, _)) => {
                parsed += 1;
                longest = longest.max(t.len());
                if t.has_groups() {
                    groups += 1;
                }
                for r in t.file_refs() {
                    let l = r.to_ascii_lowercase();
                    ovh += usize::from(l.ends_with(".ovh"));
                    bus += usize::from(l.ends_with(".bus"));
                }
                assert!(t
                    .segments
                    .iter()
                    .all(|s| !matches!(&s.vehicle, VehicleRef::File(p) if p.is_empty())));
            }
            Err(ZugError::NotAConsist { .. }) => refused.push(name),
            Err(e) => panic!("{rel}: {e}"),
        }
    }
    eprintln!(
        "parsed {parsed}, refused {refused:?}, files with group entries {groups}, longest {longest}, .ovh {ovh}, .bus {bus}"
    );
    assert!(parsed >= 100);
    assert!(
        refused.iter().all(|n| n == "RUHR"),
        "only the stray script may be refused: {refused:?}"
    );
    assert_eq!(bus, 0, "consists never name a .bus file");
    assert!(ovh > 1000);
    assert!(longest <= 64, "longest formation {longest}");
}
