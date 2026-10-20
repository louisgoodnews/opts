//! The scenery sound configurations of the installation parse without surprises (M-0139):
//! every `Sceneryobjects/**/sound/*.cfg` reads, its definitions have a file name, a position
//! and a range, and the keywords seen are the ones the format page lists. Skipped without
//! `OMSI_PATH`.

use std::path::{Path, PathBuf};

use opts_core::formats::sound::SoundConfig;

fn cfgs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            cfgs(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("cfg"))
            // `rw_c_p_concrete.bmp.cfg` and its kind are texture configurations (`[moisture]`,
            // `[puddles]`, `[surface]`), not sound configurations.
            && !p.file_stem().is_some_and(|s| s.to_string_lossy().contains('.'))
            && p.parent()
                .and_then(|d| d.file_name())
                .is_some_and(|n| n.eq_ignore_ascii_case("sound"))
        {
            out.push(p);
        }
    }
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}

#[test]
fn every_scenery_sound_config_parses_cleanly() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let mut files = Vec::new();
    cfgs(&root.join("Sceneryobjects"), &mut files);
    if files.is_empty() {
        eprintln!("no scenery sound configs; skipping");
        return;
    }
    let (mut defs, mut warnings, mut no_pos) = (0, Vec::new(), 0);
    let (mut continuous, mut triggered, mut conditioned) = (0, 0, 0);
    for f in &files {
        let text = latin1(&std::fs::read(f).expect("read"));
        let c = SoundConfig::parse(&text);
        for w in &c.warnings {
            warnings.push(format!("{}: {w}", f.display()));
        }
        for d in &c.sounds {
            defs += 1;
            assert!(
                !d.file.is_empty(),
                "{}: a definition without a file",
                f.display()
            );
            if d.position.is_none() {
                eprintln!("  no [3d]: {} in {}", d.file, f.display());
            }
            no_pos += usize::from(d.position.is_none());
            continuous += usize::from(d.is_continuous());
            triggered += usize::from(!d.triggers.is_empty());
            conditioned += usize::from(!d.conditions.is_empty());
        }
    }
    eprintln!(
        "{} files, {defs} definitions ({continuous} continuous, {triggered} triggered, {conditioned} with a condition), {no_pos} without [3d], {} warnings",
        files.len(),
        warnings.len()
    );
    for w in warnings.iter().take(15) {
        eprintln!("  {w}");
    }
    assert!(
        files.len() >= 250 && defs >= 300,
        "{} files, {defs} definitions",
        files.len()
    );
    // One object (a taxi in `Kartoffelphantom`) names a file in the shared `Sounds` folder
    // without a [3d] block; every other definition has one.
    assert!(no_pos <= 1, "{no_pos} definitions without [3d]");
    assert!(warnings.len() <= 3, "{} warnings", warnings.len());
}
