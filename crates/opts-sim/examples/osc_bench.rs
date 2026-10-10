//! Times `{frame}` of one vehicle's scripts: `osc_bench <script dir> [frames]`.
use std::path::Path;
use std::time::Instant;

use opts_core::formats::text::decode_bytes;
use opts_sim::script::{Compiler, ScriptEngine};

fn main() {
    let dir = std::env::args().nth(1).expect("script directory");
    let frames: u32 = std::env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000);
    let mut files: Vec<_> = std::fs::read_dir(Path::new(&dir))
        .expect("dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("osc")))
        .collect();
    files.sort();
    let mut c = Compiler::new();
    for f in &files {
        c.add_source(
            &f.to_string_lossy(),
            &decode_bytes(&std::fs::read(f).expect("read")),
        );
    }
    let (program, diags) = c.finish();
    println!(
        "{} files, {} blocks, {} diagnostics",
        files.len(),
        program.blocks.len(),
        diags.len()
    );
    let mut e = ScriptEngine::new(program);
    e.run_init();
    let t = Instant::now();
    for _ in 0..frames {
        e.run_frame(1.0 / 60.0, false);
    }
    let per = t.elapsed().as_secs_f64() / f64::from(frames);
    println!(
        "{:.1} µs per frame ({frames} frames), {} ops per frame",
        per * 1e6,
        e.ops_executed() / u64::from(frames)
    );
    println!("run-time diagnostics: {}", e.drain_diagnostics().len());
}
