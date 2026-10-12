//! Prints the bounding box of the `.o3d` files read from stdin, after the confident decoder.
use std::io::BufRead;

use opts_core::formats::o3d::{descramble::decode_confident, parse};

fn main() {
    for p in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(b) = std::fs::read(&p) else {
            println!("MISSING {p}");
            continue;
        };
        let Ok(mut o) = parse(&b) else { continue };
        let scrambled = o.encryption_key.is_some();
        let verdict = if scrambled {
            let r = decode_confident(&mut o);
            if r.accepted {
                "decoded"
            } else {
                "SKIPPED"
            }
        } else {
            "plain"
        };
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in &o.vertices {
            for (k, c) in [v.position.x, v.position.y, v.position.z]
                .into_iter()
                .enumerate()
            {
                lo[k] = lo[k].min(c);
                hi[k] = hi[k].max(c);
            }
        }
        println!(
            "{verdict:8} v={:6} size=({:.1},{:.1},{:.1}) {}",
            o.vertices.len(),
            hi[0] - lo[0],
            hi[1] - lo[1],
            hi[2] - lo[2],
            p.rsplit('/').next().unwrap_or("")
        );
    }
}
