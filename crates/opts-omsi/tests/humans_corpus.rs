//! Corpus test: every `.hum` in a real OMSI installation parses, and its model manifest too.
//! Skipped unless `OMSI_PATH` points at an installation.

use std::path::{Path, PathBuf};

use opts_omsi::formats::humans::HumanProfile;
use opts_omsi::formats::vehicles::model::ModelConfig;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("hum")) {
                out.push(p);
            }
        }
    }
}

#[test]
fn every_hum_and_its_model_manifest_parse() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let mut files = Vec::new();
    walk(&root.join("Humans"), &mut files);
    assert!(!files.is_empty(), "no .hum files under {}", root.display());
    for f in &files {
        let h = HumanProfile::parse(&std::fs::read(f).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        assert!(
            (1.0..2.2).contains(&h.height_m),
            "{}: implausible height {}",
            f.display(),
            h.height_m
        );
        assert!(
            h.links.neck.y < h.height_m,
            "{}: neck above crown",
            f.display()
        );
        // Model manifest relative to the .hum's directory; tolerate case differences.
        let rel = h.model.replace('\\', "/");
        let dir = f.parent().unwrap();
        let cfg = std::fs::read_dir(dir.join(Path::new(&rel).parent().unwrap_or(Path::new(""))))
            .ok()
            .and_then(|rd| {
                rd.flatten().map(|e| e.path()).find(|p| {
                    p.file_name().map(|n| n.to_string_lossy().to_lowercase())
                        == Path::new(&rel)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_lowercase())
                })
            });
        if let Some(cfg) = cfg {
            ModelConfig::parse_bytes(&std::fs::read(&cfg).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", cfg.display()));
        }
    }
}
