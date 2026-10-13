//! Tests where the animation axis of `origin_from_mesh` meshes points. Input lines (tab separated):
//! `<.o3d path> <rx> <ry> <rz>`; prints, per hypothesis, whether the resulting X axis of the frame
//! is vertical (vehicle z) — H1: frame in vehicle axes, matrix ignored; H2: matrix rotation
//! (rows = images of the local axes) applied after the origin rotation; H3: its transpose.
use std::collections::BTreeMap;
use std::io::BufRead;

use opts_core::formats::o3d::parse;

type V3 = [f32; 3];

fn rot(rx: f32, ry: f32, rz: f32, v: V3) -> V3 {
    // Rx·Ry·Rz applied to v: Rz first.
    let (sz, cz) = rz.to_radians().sin_cos();
    let v = [cz * v[0] - sz * v[1], sz * v[0] + cz * v[1], v[2]];
    let (sy, cy) = ry.to_radians().sin_cos();
    let v = [cy * v[0] + sy * v[2], v[1], -sy * v[0] + cy * v[2]];
    let (sx, cx) = rx.to_radians().sin_cos();
    [v[0], cx * v[1] - sx * v[2], sx * v[1] + cx * v[2]]
}

fn norm(v: V3) -> V3 {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
    [v[0] / l, v[1] / l, v[2] / l]
}

fn main() {
    let mut tally: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 4 {
            continue;
        }
        let (rx, ry, rz): (f32, f32, f32) = (
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
        );
        let Ok(b) = std::fs::read(f[0]) else { continue };
        let Ok(o) = parse(&b) else { continue };
        let Some(m) = o.transform else { continue };
        let a1 = rot(rx, ry, rz, [1.0, 0.0, 0.0]);
        let r = [
            norm([m[0][0], m[0][1], m[0][2]]),
            norm([m[1][0], m[1][1], m[1][2]]),
            norm([m[2][0], m[2][1], m[2][2]]),
        ];
        let h2 = [
            a1[0] * r[0][0] + a1[1] * r[1][0] + a1[2] * r[2][0],
            a1[0] * r[0][1] + a1[1] * r[1][1] + a1[2] * r[2][1],
            a1[0] * r[0][2] + a1[1] * r[1][2] + a1[2] * r[2][2],
        ];
        let h3 = [
            a1[0] * r[0][0] + a1[1] * r[0][1] + a1[2] * r[0][2],
            a1[0] * r[1][0] + a1[1] * r[1][1] + a1[2] * r[1][2],
            a1[0] * r[2][0] + a1[1] * r[2][1] + a1[2] * r[2][2],
        ];
        let vertical = |v: V3| norm(v)[2].abs() > 0.97;
        // Optional fifth column: the animation factor; separates the main steering (about 57) from small side effects.
        let big = f
            .get(4)
            .and_then(|x| x.parse::<f32>().ok())
            .map_or("", |x| {
                if x.abs() > 20.0 {
                    " factor~57"
                } else {
                    " factor small"
                }
            });
        let class = format!("rot=({rx},{ry},{rz}){big} matrix=[{:.0},{:.0},{:.0}|{:.0},{:.0},{:.0}|{:.0},{:.0},{:.0}]", r[0][0], r[0][1], r[0][2], r[1][0], r[1][1], r[1][2], r[2][0], r[2][1], r[2][2]);
        let e = tally.entry(class).or_default();
        e[0] += 1;
        e[1] += usize::from(vertical(a1));
        e[2] += usize::from(vertical(h2));
        e[3] += usize::from(vertical(h3));
    }
    println!(
        "{:90} {:>5} {:>4} {:>4} {:>4}",
        "frame rotation + matrix rotation", "n", "H1", "H2", "H3"
    );
    let mut v: Vec<_> = tally.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1[0]));
    let tot = v.iter().fold([0usize; 4], |mut a, e| {
        for k in 0..4 {
            a[k] += e.1[k];
        }
        a
    });
    for (k, e) in v.iter().take(18) {
        println!("{k:70} {:5} {:4} {:4} {:4}", e[0], e[1], e[2], e[3]);
    }
    println!(
        "{:90} {:5} {:4} {:4} {:4}",
        "TOTAL", tot[0], tot[1], tot[2], tot[3]
    );
}
