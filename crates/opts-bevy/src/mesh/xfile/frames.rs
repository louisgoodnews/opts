//! The transforms of the `.x` frame hierarchy: each sub-mesh with its frame's world transform.


/// Row-vector matrix product `a * b` (apply `a` first, then `b`).
fn mul4(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

pub(super) fn collect_frame_submeshes<'a>(
    frame: &'a opts_core::formats::x::XFrame,
    parent: Option<[[f32; 4]; 4]>,
    out: &mut Vec<(&'a opts_core::formats::x::XSubMesh, Option<[[f32; 4]; 4]>)>,
) {
    let world = match (frame.transform, parent) {
        (Some(local), Some(p)) => Some(mul4(&local, &p)),
        (Some(local), None) => Some(local),
        (None, p) => p,
    };
    for m in &frame.meshes {
        out.push((m, world));
    }
    for child in &frame.children {
        collect_frame_submeshes(child, world, out);
    }
}
