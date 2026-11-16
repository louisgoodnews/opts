//! The triangle, winding and texture-coordinate checks of the mesh audit: degenerate and huge
//! triangles, normal agreement, and UV chaos.

use super::percentile;
use crate::formats::o3d::{O3d, Vertex};

fn check_uv_chaos(
    uv_ratio: &mut [f32],
    vertices: &[Vertex],
    tag: &str,
    codes: &mut Vec<String>,
    metrics: &mut Vec<String>,
) {
    if uv_ratio.len() >= 8 {
        let m = percentile(uv_ratio, 0.5).max(1e-6);
        let bad = uv_ratio
            .iter()
            .filter(|&&r| r > m * 40.0 && r > 2.0)
            .count();
        let frac = bad as f32 / uv_ratio.len() as f32;
        metrics.push(format!("{tag}uvchaos={frac:.2}"));
        if frac > 0.10 {
            codes.push(format!("{tag}UV_CHAOS"));
        }
    }
    if vertices.iter().any(|v| v.uv.abs().max_element() > 1000.0) {
        codes.push(format!("{tag}UV_HUGE"));
    }
}

pub(super) fn check_triangles_and_uv(o: &O3d, tag: &str, codes: &mut Vec<String>, metrics: &mut Vec<String>) {
    let nt = o.indices.len();
    let (mut degen, mut huge, mut flip, mut agree_n) = (0usize, 0usize, 0usize, 0usize);
    let mut edges: Vec<f32> = Vec::with_capacity(nt);
    let mut uv_ratio: Vec<f32> = Vec::with_capacity(nt);
    let mut tri_edges = Vec::with_capacity(nt);
    for t in &o.indices {
        let [a, b, c] = t.map(|i| &o.vertices[i as usize]);
        let (e1, e2) = (b.position - a.position, c.position - a.position);
        let area = e1.cross(e2).length();
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] || area < 1e-9 {
            degen += 1;
            continue;
        }
        let maxe = e1
            .length()
            .max(e2.length())
            .max((c.position - b.position).length());
        edges.push(maxe);
        tri_edges.push(maxe);
        let g = e1.cross(e2) / area;
        let vn = a.normal + b.normal + c.normal;
        if vn.length() > 1e-3 {
            let cs = g.dot(vn.normalize());
            if cs.abs() > 0.5 {
                agree_n += 1;
                if cs < 0.0 {
                    flip += 1;
                }
            }
        }
        let uvm = (b.uv - a.uv)
            .length()
            .max((c.uv - a.uv).length())
            .max((c.uv - b.uv).length());
        if maxe > 1e-4 {
            uv_ratio.push(uvm / maxe);
        }
    }
    if degen * 20 > nt {
        codes.push(format!("{tag}DEGENERATE_TRIS"));
    }
    if degen > 0 {
        metrics.push(format!("{tag}degen={degen}/{nt}"));
    }
    let med_edge = percentile(&mut edges.clone(), 0.5);
    for &e in &tri_edges {
        if e > med_edge * 25.0 && e > 100.0 {
            huge += 1;
        }
    }
    if huge > 0 {
        metrics.push(format!("{tag}huge_tris={huge}/{nt}"));
        if huge * 50 > nt || huge >= 3 {
            codes.push(format!("{tag}HUGE_TRIS"));
        }
    }
    if agree_n >= 8 {
        let f = flip as f32 / agree_n as f32;
        metrics.push(format!("{tag}flip={f:.2}"));
        if (0.25..0.75).contains(&f) {
            codes.push(format!("{tag}WINDING_MIXED"));
        }
    }
    check_uv_chaos(&mut uv_ratio, &o.vertices, tag, codes, metrics);
}
