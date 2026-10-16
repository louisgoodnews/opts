//! Prints, for the `.o3d` files read from stdin, the per-axis extent after the confident decoder,
//! the axes whose minimum lies on zero (the floor axes, see `O3d::floor_axes`) and the vertex
//! layout. `DUMP=1` also prints every vertex (`V x y z`) and face (`F a b c`).
use std::io::BufRead;

use opts_core::formats::o3d::{descramble::decode_confident, parse};

fn main() {
    for p in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(b) = std::fs::read(&p) else {
            println!("MISSING {p}");
            continue;
        };
        let Ok(mut o) = parse(&b) else { continue };
        let verdict = if o.encryption_key.is_some() {
            if decode_confident(&mut o).accepted {
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
        if std::env::var_os("DUMP").is_some() {
            for v in &o.vertices {
                println!("V {} {} {}", v.position.x, v.position.y, v.position.z);
            }
            for t in &o.indices {
                println!("F {} {} {}", t[0], t[1], t[2]);
            }
        }
        let name = p.rsplit('/').next().unwrap_or(&p);
        println!(
            "{verdict:8} v={:5} x=[{:6.1},{:6.1}] y=[{:6.1},{:6.1}] z=[{:6.1},{:6.1}] floor={:?} layout={:?} {name}",
            o.vertices.len(),
            lo[0],
            hi[0],
            lo[1],
            hi[1],
            lo[2],
            hi[2],
            o.floor_axes(),
            o.vertex_layout(),
        );
    }
}
