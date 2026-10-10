//! Runs the confident decoder on the `.o3d` files read from stdin and prints timing and verdicts.
use std::io::BufRead;
use std::time::Instant;

use opts_core::formats::o3d::{descramble::decode_confident, parse};

fn main() {
    let (mut acc, mut tot, mut ms_total) = (0, 0, 0u128);
    for p in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(b) = std::fs::read(&p) else { continue };
        let Ok(mut o) = parse(&b) else { continue };
        if o.encryption_key.is_none() {
            continue;
        }
        let t = Instant::now();
        let r = decode_confident(&mut o);
        let ms = t.elapsed().as_millis();
        ms_total += ms;
        tot += 1;
        acc += usize::from(r.accepted);
        println!(
            "{} v={} {} agree={:.3} coh={:.3} dropped={} uv(fixed {} clean {} unres {}) {ms} ms",
            if r.accepted { "OK  " } else { "SKIP" },
            o.vertices.len(),
            p.rsplit('/').next().unwrap_or(""),
            r.agreement,
            r.coherence,
            r.dropped_faces,
            r.uv.corrected,
            r.uv.clean,
            r.uv.unresolved
        );
    }
    println!("accepted {acc}/{tot}, total {ms_total} ms");
}
