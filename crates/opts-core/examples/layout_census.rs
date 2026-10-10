//! Classifies every `.o3d` (paths on stdin) by vertex layout and header version.
use opts_core::formats::o3d::VertexLayout;
use std::collections::BTreeMap;
use std::io::BufRead;

fn main() {
    let mut hist: BTreeMap<(u32, &'static str), u32> = BTreeMap::new();
    for line in std::io::stdin().lock().lines() {
        let Ok(p) = line else { continue };
        let Ok(b) = std::fs::read(&p) else { continue };
        let Ok(o) = opts_core::formats::o3d::parse(&b) else {
            continue;
        };
        let class = match o.vertex_layout() {
            VertexLayout::Standard => "standard",
            VertexLayout::AxisCycled => "axis-cycled",
            VertexLayout::RecomputedNormals => "y-up, normals recomputed",
            VertexLayout::Unreadable => "unreadable",
        };
        *hist.entry((o.version, class)).or_default() += 1;
    }
    for ((v, c), n) in hist {
        println!("v{v:<2} {c:12} {n}");
    }
}
