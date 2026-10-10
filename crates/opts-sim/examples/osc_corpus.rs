//! Compiles every `.osc` below a directory and reports diagnostics (corpus check for the script
//! compiler): `cargo run --release -p opts-sim --example osc_corpus -- <OMSI dir>`.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use opts_core::formats::text::decode_bytes;
use opts_sim::script::{Compiler, Severity};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("osc")) {
            out.push(p);
        }
    }
}

fn main() {
    let dir = std::env::args().nth(1).expect("OMSI directory");
    let mut files = Vec::new();
    walk(Path::new(&dir), &mut files);
    let (mut blocks, mut errors, mut warnings) = (0usize, 0usize, 0usize);
    let mut kinds: HashMap<String, usize> = HashMap::new();
    let mut samples: Vec<String> = Vec::new();
    for f in &files {
        let text = decode_bytes(&std::fs::read(f).expect("read"));
        let mut c = Compiler::new();
        c.add_source(&f.to_string_lossy(), &text);
        let (program, diags) = c.finish();
        blocks += program.blocks.len();
        for d in diags {
            match d.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
            }
            let key = d
                .message
                .split('`')
                .nth(1)
                .map_or_else(|| d.message.clone(), |t| format!("`{t}`"));
            *kinds.entry(key).or_default() += 1;
            if samples.len() < 8 && d.severity == Severity::Error {
                samples.push(format!("{}:{} {}", d.file, d.line, d.message));
            }
        }
    }
    println!(
        "{} files, {blocks} blocks, {errors} errors, {warnings} warnings",
        files.len()
    );
    let mut top: Vec<_> = kinds.into_iter().collect();
    top.sort_by_key(|b| std::cmp::Reverse(b.1));
    for (k, n) in top.iter().take(400) {
        println!("{n:7}  {k}");
    }
    for s in samples {
        println!("ERR {s}");
    }
}
