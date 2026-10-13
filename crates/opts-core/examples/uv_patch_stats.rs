//! Coverage ceiling of plane-based UV decoding (P-1013): for every vertex of the plain twins, the
//! size of the largest set of coplanar vertices that share one affine UV map with a face it belongs
//! to. Vertices in small patches cannot be verified by a plane model. Usage: `uv_patch_stats <pairs.tsv> [five]`.
use glam::Vec3;
use opts_core::formats::o3d::parse;

fn main() {
    let mut args = std::env::args().skip(1);
    let text = std::fs::read_to_string(args.next().unwrap()).unwrap();
    let five_only = args.next().as_deref() == Some("five");
    let mut hist = [0usize; 8]; // patch size buckets: 1-2, 3-5, 6-11, 12-23, 24-47, 48-99, 100+
    let mut total = 0usize;
    for l in text.lines().skip(1).take(150) {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 8 || f[7] != "true" {
            continue;
        }
        let Ok(o) = parse(&std::fs::read(f[1]).unwrap()) else { continue };
        let n = o.vertices.len();
        if (five_only && n % 5 != 0) || n > 6000 {
            continue;
        }
        let pos: Vec<Vec3> = o.vertices.iter().map(|v| v.position).collect();
        let mut best = vec![0usize; n];
        for t in &o.indices {
            let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
            if a >= n || b >= n || c >= n {
                continue;
            }
            let nrm = (pos[b] - pos[a]).cross(pos[c] - pos[a]);
            if nrm.length_squared() < 1e-10 {
                continue;
            }
            let nrm = nrm.normalize();
            let e1 = (pos[b] - pos[a]).normalize();
            let e2 = nrm.cross(e1);
            let st = |q: Vec3| ((q - pos[a]).dot(e1), (q - pos[a]).dot(e2));
            let (bb, cc) = (st(pos[b]), st(pos[c]));
            let det = bb.0 * cc.1 - bb.1 * cc.0;
            if det.abs() < 1e-9 {
                continue;
            }
            let uv = |i: usize| [o.vertices[i].uv.x, o.vertices[i].uv.y];
            let mut members = Vec::new();
            for i in 0..n {
                if (pos[i] - pos[a]).dot(nrm).abs() > 1e-4 {
                    continue;
                }
                let (s, tt) = st(pos[i]);
                let (l1, l2) = ((s * cc.1 - tt * cc.0) / det, (bb.0 * tt - bb.1 * s) / det);
                let lam = [1.0 - l1 - l2, l1, l2];
                let ok = (0..2).all(|k| {
                    let pred = lam[0] * uv(a)[k] + lam[1] * uv(b)[k] + lam[2] * uv(c)[k];
                    (pred - uv(i)[k]).abs() < 2e-4
                });
                if ok {
                    members.push(i);
                }
            }
            for &i in &members {
                best[i] = best[i].max(members.len());
            }
        }
        for &b in &best {
            let k = match b { 0..=2 => 0, 3..=5 => 1, 6..=11 => 2, 12..=23 => 3, 24..=47 => 4, 48..=99 => 5, _ => 6 };
            hist[k] += 1;
            total += 1;
        }
    }
    let names = ["1-2", "3-5", "6-11", "12-23", "24-47", "48-99", "100+"];
    for (i, h) in hist.iter().take(7).enumerate() {
        println!("largest affine patch of the vertex {:>6}: {:5.1}% of vertices", names[i], 100.0 * *h as f64 / total as f64);
    }
}
