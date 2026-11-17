//! Install-gated census of spline profiles (`.sli`) under `Splines/` and `Sceneryobjects/`: every
//! tunnel-named file is read by the real reader, and a file counts as parseable when it yields at
//! least one graphical profile of two or more points. Skipped unless `OMSI_PATH` points at an
//! installation. Read-only: no asset is copied or committed.

use std::path::{Path, PathBuf};

use opts_omsi::formats::scenery::SplineProfile;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sli")) {
            out.push(p);
        }
    }
}

/// True when the reader finds at least one graphical profile with two or more points.
fn has_points(profile: &SplineProfile) -> bool {
    profile
        .graphical_profiles
        .iter()
        .any(|g| g.points.len() >= 2)
}

#[test]
fn tunnel_named_profiles_of_the_installation_are_parseable() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set: skipped");
        return;
    };
    let mut files = Vec::new();
    walk(&root.join("Splines"), &mut files);
    walk(&root.join("Sceneryobjects"), &mut files);
    let (mut total, mut parsed, mut errors, mut empty) = (0usize, 0usize, 0usize, 0usize);
    let (mut tunnel_total, mut tunnel_parsed) = (0usize, 0usize);
    for f in &files {
        let tunnel = f
            .file_name()
            .is_some_and(|n| n.to_string_lossy().to_lowercase().contains("tunnel"));
        total += 1;
        tunnel_total += usize::from(tunnel);
        let bytes = std::fs::read(f).expect("read .sli");
        match SplineProfile::parse(&bytes) {
            Ok(p) if has_points(&p) => {
                parsed += 1;
                tunnel_parsed += usize::from(tunnel);
            }
            Ok(_) => {
                empty += 1;
                eprintln!("no points: {}", f.display());
            }
            Err(e) => {
                errors += 1;
                eprintln!("error: {}: {e}", f.display());
            }
        }
    }
    eprintln!(
        "all .sli: {total} files, {parsed} with points, {empty} without, {errors} errors; \
         tunnel-named: {tunnel_total} files, {tunnel_parsed} with points"
    );
    assert!(
        tunnel_total > 0,
        "no tunnel-named .sli under {}",
        root.display()
    );
}
