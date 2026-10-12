//! Prints the matrix chunk and the vertex bounds of the `.o3d` files read from stdin.
use std::io::BufRead;

use opts_core::formats::o3d::parse;

fn main() {
    for p in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(b) = std::fs::read(&p) else { continue };
        let Ok(o) = parse(&b) else { continue };
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
            "{} matrix={:?}\n   bounds centre=({:.3},{:.3},{:.3}) size=({:.3},{:.3},{:.3})",
            p.rsplit('/').next().unwrap_or(""),
            o.transform,
            (lo[0] + hi[0]) / 2.0,
            (lo[1] + hi[1]) / 2.0,
            (lo[2] + hi[2]) / 2.0,
            hi[0] - lo[0],
            hi[1] - lo[1],
            hi[2] - lo[2]
        );
    }
}
