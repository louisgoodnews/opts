//! Plausibility audit of a parsed mesh (P-1014), shared by the `mesh_audit` and
//! `mesh_detector_eval` examples and by regression tests. See `docs/src/quality/mesh-issue-registry.md`
//! for the meaning of each code.

use glam::Vec3;

use super::O3d;

fn percentile(v: &mut [f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f32 * q) as usize]
}

/// Geometric and structural plausibility checks on a (possibly decoded) mesh: non-finite values,
/// index and material-index range, empty meshes, bounding-box extent, exploded vertices, degenerate
/// and huge triangles, winding disagreement with the stored normals and texture-coordinate chaos.
///
/// Each finding appends a code (prefixed with `tag`) to `codes` and the measurement to `metrics`.
/// The winding check is skipped when `o.descrambled` (decoded meshes carry no trustworthy normals).
pub fn check(o: &O3d, tag: &str, codes: &mut Vec<String>, metrics: &mut Vec<String>) {
    let nv = o.vertices.len();
    let nt = o.indices.len();
    let finite = |v: Vec3| v.is_finite();
    if o.vertices
        .iter()
        .any(|v| !finite(v.position) || !finite(v.normal) || !v.uv.is_finite())
    {
        codes.push(format!("{tag}NONFINITE"));
    }
    if o.indices
        .iter()
        .any(|t| t.iter().any(|&i| i as usize >= nv))
    {
        codes.push(format!("{tag}INDEX_OOR"));
        return;
    }
    if o.face_materials.len() != nt {
        codes.push(format!("{tag}FACEMAT_LEN"));
    }
    if !o.materials.is_empty()
        && o.face_materials
            .iter()
            .any(|&m| m as usize >= o.materials.len())
    {
        codes.push(format!("{tag}MATIDX_OOR"));
    }
    if nv == 0 || nt == 0 {
        codes.push(format!("{tag}EMPTY"));
        return;
    }
    // bounding box and outliers
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for v in &o.vertices {
        if finite(v.position) {
            lo = lo.min(v.position);
            hi = hi.max(v.position);
        }
    }
    let diag = (hi - lo).length();
    metrics.push(format!("{tag}diag={diag:.1}"));
    if diag > 3000.0 {
        codes.push(format!("{tag}HUGE_BBOX"));
    }
    let mut med = [0.0f32; 3];
    for (k, m) in med.iter_mut().enumerate() {
        let mut c: Vec<f32> = o
            .vertices
            .iter()
            .map(|v| v.position[k])
            .filter(|x| x.is_finite())
            .collect();
        *m = percentile(&mut c, 0.5);
    }
    let med = Vec3::from(med);
    let d: Vec<f32> = o
        .vertices
        .iter()
        .map(|v| (v.position - med).length())
        .filter(|x| x.is_finite())
        .collect();
    let r90 = percentile(&mut d.clone(), 0.9);
    let out = d.iter().filter(|&&x| x > r90 * 8.0 + 5.0).count();
    if out > 0 {
        metrics.push(format!("{tag}outliers={out}/{nv}"));
        if out * 100 >= nv.max(20) || out >= 3 {
            codes.push(format!("{tag}OUTLIER_VERTS"));
        }
    }
    // triangles
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
    if agree_n >= 8 && !o.descrambled {
        let f = flip as f32 / agree_n as f32;
        metrics.push(format!("{tag}flip={f:.2}"));
        if (0.25..0.75).contains(&f) {
            codes.push(format!("{tag}WINDING_MIXED"));
        }
    }
    // uv chaos
    if uv_ratio.len() >= 8 {
        let m = percentile(&mut uv_ratio.clone(), 0.5).max(1e-6);
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
    if o.vertices.iter().any(|v| v.uv.abs().max_element() > 1000.0) {
        codes.push(format!("{tag}UV_HUGE"));
    }
}
