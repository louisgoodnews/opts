//! Histogram of the rotation part of the `.o3d` matrix chunk over the files below a directory:
//! `o3d_matrix_census <dir>`.
use std::collections::BTreeMap;

use opts_core::formats::o3d::parse;

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("o3d")) {
            out.push(p);
        }
    }
}

fn main() {
    let dir = std::env::args().nth(1).expect("dir");
    let mut files = Vec::new();
    walk(std::path::Path::new(&dir), &mut files);
    let mut hist: BTreeMap<String, usize> = BTreeMap::new();
    let (mut none, mut total) = (0, 0);
    for p in files {
        let Ok(b) = std::fs::read(&p) else { continue };
        let Ok(o) = parse(&b) else { continue };
        total += 1;
        match o.transform {
            None => none += 1,
            Some(m) => {
                let r = |v: f32| (v * 100.0).round() / 100.0 + 0.0;
                let key = format!(
                    "{:?} {:?} {:?}",
                    [r(m[0][0]), r(m[0][1]), r(m[0][2])],
                    [r(m[1][0]), r(m[1][1]), r(m[1][2])],
                    [r(m[2][0]), r(m[2][1]), r(m[2][2])]
                );
                *hist.entry(key).or_default() += 1;
            }
        }
    }
    println!("{total} files, {none} without a matrix chunk");
    let mut v: Vec<_> = hist.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1));
    for (k, n) in v.into_iter().take(12) {
        println!("{n:7}  {k}");
    }
}
