//! Ground truth for rotation axes of disc- or ring-shaped meshes (wheels, rims, steering wheels):
//! the axis the part turns about is the normal of its flat shape, the direction of least
//! variance of its vertices. Input lines (tab separated): `<.o3d path> <rx> <ry> <rz> <label>`;
//! prints, per label and matrix class, how many animation axes agree (|cos| > 0.95) with the
//! geometric normal under H1 (origin rotation only), H2 (matrix rows applied after the origin
//! rotation) and H3 (its transpose).
#![allow(clippy::needless_range_loop)] // research prototype

use std::collections::BTreeMap;
use std::io::BufRead;

use opts_omsi::formats::o3d::parse;

type V3 = [f32; 3];

fn rot(rx: f32, ry: f32, rz: f32, v: V3) -> V3 {
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

fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Direction of least variance by Jacobi eigen decomposition of the covariance.
fn flat_normal(points: &[V3]) -> V3 {
    let n = points.len() as f32;
    let mean = points.iter().fold([0.0; 3], |a, p| {
        [a[0] + p[0] / n, a[1] + p[1] / n, a[2] + p[2] / n]
    });
    let mut c = [[0.0f32; 3]; 3];
    for p in points {
        let d = [p[0] - mean[0], p[1] - mean[1], p[2] - mean[2]];
        for i in 0..3 {
            for j in 0..3 {
                c[i][j] += d[i] * d[j] / n;
            }
        }
    }
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..30 {
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if c[p][q].abs() < 1e-12 {
                continue;
            }
            let theta = (c[q][q] - c[p][p]) / (2.0 * c[p][q]);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
            let (cs, sn) = (1.0 / (t * t + 1.0).sqrt(), t / (t * t + 1.0).sqrt());
            for k in 0..3 {
                let (a, b) = (c[k][p], c[k][q]);
                c[k][p] = cs * a - sn * b;
                c[k][q] = sn * a + cs * b;
            }
            for k in 0..3 {
                let (a, b) = (c[p][k], c[q][k]);
                c[p][k] = cs * a - sn * b;
                c[q][k] = sn * a + cs * b;
            }
            for k in 0..3 {
                let (a, b) = (v[k][p], v[k][q]);
                v[k][p] = cs * a - sn * b;
                v[k][q] = sn * a + cs * b;
            }
        }
    }
    let i = (0..3).min_by(|&a, &b| c[a][a].total_cmp(&c[b][b])).unwrap();
    norm([v[0][i], v[1][i], v[2][i]])
}

fn main() {
    let mut tally: BTreeMap<String, [usize; 5]> = BTreeMap::new();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 {
            continue;
        }
        let (rx, ry, rz): (f32, f32, f32) = (
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
        );
        let Ok(b) = std::fs::read(f[0]) else { continue };
        let Ok(mut o) = parse(&b) else { continue };
        if std::env::var_os("AXIS_SKIP_SCRAMBLED").is_some() && o.encryption_key.is_some() {
            continue;
        }
        if o.encryption_key.is_some() {
            opts_omsi::formats::o3d::exact::decode(&mut o);
        }
        let Some(m) = o.transform else { continue };
        if o.vertices.len() < 12 {
            continue;
        }
        // o3d (x lateral, y up, z forward) -> vehicle axes (x lateral, y forward, z up)
        let pts: Vec<V3> = o
            .vertices
            .iter()
            .map(|v| [v.position.x, v.position.z, v.position.y])
            .collect();
        let n = flat_normal(&pts);
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
        let h3 = [dot(a1, r[0]), dot(a1, r[1]), dot(a1, r[2])];
        if std::env::var_os("AXIS_VERBOSE").is_some() {
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for p in &pts {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
            eprintln!("verts={} bbox lo={lo:?} hi={hi:?}", pts.len());
            eprintln!("normal={n:?} rows={r:?} h1={a1:?} h3={h3:?}");
        }
        // H4: the matrix rows are the object's axes in o3d coordinates (y up, z forward): convert
        // the origin axis to o3d, express it in the parent with the rows, convert back.
        let c = |v: V3| [v[0], v[2], v[1]];
        let ca = c(a1);
        let h4 = c([
            ca[0] * r[0][0] + ca[1] * r[1][0] + ca[2] * r[2][0],
            ca[0] * r[0][1] + ca[1] * r[1][1] + ca[2] * r[2][1],
            ca[0] * r[0][2] + ca[1] * r[1][2] + ca[2] * r[2][2],
        ]);
        let ok = |a: V3| dot(norm(a), n).abs() > 0.95;
        let class = format!(
            "{} rot=({rx},{ry},{rz}) matrix=[{:.0},{:.0},{:.0}|{:.0},{:.0},{:.0}|{:.0},{:.0},{:.0}]",
            f[4], r[0][0], r[0][1], r[0][2], r[1][0], r[1][1], r[1][2], r[2][0], r[2][1], r[2][2]
        );
        let det = dot(
            r[0],
            [
                r[1][1] * r[2][2] - r[1][2] * r[2][1],
                r[1][2] * r[2][0] - r[1][0] * r[2][2],
                r[1][0] * r[2][1] - r[1][1] * r[2][0],
            ],
        );
        let ident =
            (0..3).all(|i| (0..3).all(|j| (r[i][j] - f32::from(u8::from(i == j))).abs() < 0.02));
        let group = format!(
            "AGG {} {}",
            f[4],
            if ident {
                "identity"
            } else if det > 0.0 {
                "rotation (det +1)"
            } else {
                "reflection (det -1)"
            }
        );
        for key in [group, class] {
            let e = tally.entry(key).or_default();
            e[0] += 1;
            e[1] += usize::from(ok(a1));
            e[2] += usize::from(ok(h2));
            e[3] += usize::from(ok(h3));
            e[4] += usize::from(ok(h4));
        }
    }
    println!(
        "{:96} {:>5} {:>4} {:>4} {:>4} {:>4}",
        "label rotation matrix", "n", "H1", "H2", "H3", "H4"
    );
    let mut v: Vec<_> = tally.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1[0]));
    for (k, e) in v.iter().take(100000) {
        println!(
            "{k:96} {:5} {:4} {:4} {:4} {:4}",
            e[0], e[1], e[2], e[3], e[4]
        );
    }
}
