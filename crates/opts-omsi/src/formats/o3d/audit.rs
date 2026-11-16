//! Plausibility audit of a parsed mesh (P-1014), shared by the `mesh_audit` and
//! `mesh_detector_eval` examples and by regression tests. See `docs/src/quality/mesh-issue-registry.md`
//! for the meaning of each code.

use glam::Vec3;

use super::O3d;

mod triangles;

use triangles::check_triangles_and_uv;

fn percentile(v: &mut [f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f32 * q) as usize]
}

fn check_structure(o: &O3d, tag: &str, codes: &mut Vec<String>) -> bool {
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
        return false;
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
        return false;
    }
    true
}

fn check_bbox_and_outliers(o: &O3d, tag: &str, codes: &mut Vec<String>, metrics: &mut Vec<String>) {
    let nv = o.vertices.len();
    let finite = |v: Vec3| v.is_finite();
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
}


/// Geometric and structural plausibility checks on a (possibly decoded) mesh: non-finite values,
/// index and material-index range, empty meshes, bounding-box extent, exploded vertices, degenerate
/// and huge triangles, winding disagreement with the stored normals and texture-coordinate chaos.
///
/// Each finding appends a code (prefixed with `tag`) to `codes` and the measurement to `metrics`.
pub fn check(o: &O3d, tag: &str, codes: &mut Vec<String>, metrics: &mut Vec<String>) {
    if !check_structure(o, tag, codes) {
        return;
    }
    check_bbox_and_outliers(o, tag, codes, metrics);
    check_triangles_and_uv(o, tag, codes, metrics);
}
