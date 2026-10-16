//! Software contact sheet of `.o3d` meshes (P-1014 visual triage without a GPU).
//!
//! Reads mesh paths from stdin, renders each one flat-shaded from a fixed three-quarter view into a
//! cell of a grid and writes a binary PPM (convert with ImageMagick). Front faces are grey-blue,
//! back faces red (wrong winding shows up as red), degenerate meshes as an empty cell, skipped
//! scrambled meshes as a dark-grey cell. The index to path mapping is printed on stdout.
//!
//! Usage: `find ... -name '*.o3d' | mesh_sheet out.ppm [--cols 6] [--cell 200] [--decode]`
//! The first line of the output lists `cell<TAB>path<TAB>status`.

use std::io::BufRead;

use glam::{Vec2, Vec3};
use opts_core::formats::o3d::{parse, O3d, VertexLayout};

fn to_view(o: &O3d, v: Vec3) -> Vec3 {
    match o.vertex_layout() {
        VertexLayout::AxisCycled => Vec3::new(-v.y, v.x, v.z),
        _ => v,
    }
}

fn render(o: &O3d, cell: usize) -> Vec<[u8; 3]> {
    let mut img = vec![[24u8, 24, 28]; cell * cell];
    let pts: Vec<Vec3> = o.vertices.iter().map(|v| to_view(o, v.position)).collect();
    if pts.is_empty() || pts.iter().any(|p| !p.is_finite()) {
        return img;
    }
    // three-quarter view: yaw 35 deg, pitch 25 deg
    let (sy, cy) = 35f32.to_radians().sin_cos();
    let (sp, cp) = 25f32.to_radians().sin_cos();
    let proj = |p: Vec3| -> Vec3 {
        let x = p.x * cy + p.z * sy;
        let z = -p.x * sy + p.z * cy;
        let y = p.y * cp - z * sp;
        let d = p.y * sp + z * cp;
        Vec3::new(x, y, d)
    };
    let pr: Vec<Vec3> = pts.iter().map(|&p| proj(p)).collect();
    // robust frame: central 98 % of the vertices, so a few exploded vertices stay visible as spikes
    // but cannot shrink the model to a dot; the whole bbox is used when it is not much larger.
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in &pr {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let mut xs: Vec<f32> = pr.iter().map(|p| p.x).collect();
    let mut ys: Vec<f32> = pr.iter().map(|p| p.y).collect();
    xs.sort_by(|a, b| a.total_cmp(b));
    ys.sort_by(|a, b| a.total_cmp(b));
    let q = |v: &Vec<f32>, f: f32| v[((v.len() - 1) as f32 * f) as usize];
    let (x0, x1, y0, y1) = (q(&xs, 0.01), q(&xs, 0.99), q(&ys, 0.01), q(&ys, 0.99));
    let pad = 0.1;
    let (ex, ey) = ((x1 - x0).max(1e-3), (y1 - y0).max(1e-3));
    let (fx0, fx1, fy0, fy1) = (x0 - ex * pad, x1 + ex * pad, y0 - ey * pad, y1 + ey * pad);
    let scale = (cell as f32 - 4.0) / (fx1 - fx0).max(fy1 - fy0).max(1e-3);
    let to_px = |p: Vec3| Vec2::new((p.x - fx0) * scale + 2.0, (fy1 - p.y) * scale + 2.0);
    let mut zb = vec![f32::MAX; cell * cell];
    let light = Vec3::new(0.4, 0.8, -0.5).normalize();
    for t in &o.indices {
        let (Some(&a), Some(&b), Some(&c)) = (
            pr.get(t[0] as usize),
            pr.get(t[1] as usize),
            pr.get(t[2] as usize),
        ) else {
            continue;
        };
        let n3 = (pts[t[1] as usize] - pts[t[0] as usize])
            .cross(pts[t[2] as usize] - pts[t[0] as usize]);
        let nl = n3.length();
        if nl < 1e-12 {
            continue;
        }
        let n = n3 / nl;
        // screen-space orientation decides front/back (view looks along +d)
        let (pa, pb, pc) = (to_px(a), to_px(b), to_px(c));
        let area = (pb - pa).perp_dot(pc - pa);
        let back = area < 0.0;
        let shade = (n.dot(light).abs() * 0.7 + 0.3).clamp(0.0, 1.0);
        let col = if back {
            [
                (200.0 * shade) as u8,
                (50.0 * shade) as u8,
                (50.0 * shade) as u8,
            ]
        } else {
            [
                (120.0 * shade) as u8,
                (150.0 * shade) as u8,
                (200.0 * shade) as u8,
            ]
        };
        if area.abs() < 1e-6 {
            continue;
        }
        let (minx, maxx) = (
            pa.x.min(pb.x).min(pc.x).floor().max(0.0),
            pa.x.max(pb.x).max(pc.x).ceil().min(cell as f32 - 1.0),
        );
        let (miny, maxy) = (
            pa.y.min(pb.y).min(pc.y).floor().max(0.0),
            pa.y.max(pb.y).max(pc.y).ceil().min(cell as f32 - 1.0),
        );
        if minx > maxx || miny > maxy {
            continue;
        }
        for y in miny as usize..=maxy as usize {
            for x in minx as usize..=maxx as usize {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let w0 = (pb - p).perp_dot(pc - p) / area;
                let w1 = (pc - p).perp_dot(pa - p) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                    continue;
                }
                let d = w0 * a.z + w1 * b.z + w2 * c.z;
                let i = y * cell + x;
                if d < zb[i] {
                    zb[i] = d;
                    img[i] = col;
                }
            }
        }
    }
    img
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("output .ppm");
    let (mut cols, mut cell, mut decode) = (6usize, 200usize, false);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--cols" => cols = args.next().and_then(|s| s.parse().ok()).unwrap_or(6),
            "--cell" => cell = args.next().and_then(|s| s.parse().ok()).unwrap_or(200),
            "--decode" => decode = true,
            _ => {}
        }
    }
    let paths: Vec<String> = std::io::stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .collect();
    let rows = paths.len().div_ceil(cols).max(1);
    let (w, h) = (cols * cell, rows * cell);
    let mut sheet = vec![[0u8, 0, 0]; w * h];
    for (k, p) in paths.iter().enumerate() {
        let mut status = String::from("ok");
        let img = match std::fs::read(p).ok().and_then(|b| parse(&b).ok()) {
            None => {
                status = "unparsable".into();
                vec![[60u8, 0, 60]; cell * cell]
            }
            Some(mut o) => {
                if decode && o.encryption_key.is_some() {
                    let decoded = opts_core::formats::o3d::exact::decode(&mut o).is_some();
                    status = format!("decoded={decoded}");
                }
                if o.vertex_layout() == VertexLayout::Unreadable {
                    status = "skipped-scrambled".into();
                    vec![[70u8, 70, 70]; cell * cell]
                } else {
                    render(&o, cell)
                }
            }
        };
        let (cx, cy) = ((k % cols) * cell, (k / cols) * cell);
        for y in 0..cell {
            for x in 0..cell {
                let edge = x == 0 || y == 0;
                sheet[(cy + y) * w + cx + x] = if edge {
                    [255, 255, 0]
                } else {
                    img[y * cell + x]
                };
            }
        }
        println!("{k}\t{p}\t{status}");
    }
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in sheet {
        ppm.extend_from_slice(&px);
    }
    std::fs::write(out, ppm).unwrap();
}
