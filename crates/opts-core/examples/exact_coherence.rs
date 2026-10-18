//! Scratch tool of M-0066: how coherent is the exactly decoded mesh? Prints, per keyed file, the
//! mean triangle edge divided by the bounding-box diagonal (small: a mesh; near 0.3 and up: a
//! scramble). Usage: exact_coherence <file.o3d>...
use opts_core::formats::o3d::{exact, parse};

fn main() {
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(mut m) = parse(&bytes) else { continue };
        if m.encryption_key.is_none() || m.indices.len() < 20 {
            continue;
        }
        let key = m.encryption_key;
        let (v, f) = (m.version, m.header_flags);
        exact::decode(&mut m);
        let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
        for x in &m.vertices {
            lo = lo.min(x.position);
            hi = hi.max(x.position);
        }
        let diag = (hi - lo).length().max(1e-3);
        let mut sum = 0.0f32;
        for t in &m.indices {
            let p = |i: u32| m.vertices[i as usize].position;
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            sum += (a - b).length() + (b - c).length() + (c - a).length();
        }
        let ratio = sum / (3.0 * m.indices.len() as f32 * diag);
        println!(
            "{ratio:.3}\tv{v} flags={f:#x} key={key:?} nv={} {path}",
            m.vertices.len()
        );
    }
}
