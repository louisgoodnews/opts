//! The synthetic test object's mesh: an asymmetric L-shaped solid as a text `.x`.

/// One box as double-sided quads (both windings, so culling never matters).
fn quads(min: [f32; 3], max: [f32; 3], out: &mut Vec<[[f32; 3]; 4]>) {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let faces = [
        [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
        [[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]],
        [[x0, y0, z0], [x0, y1, z0], [x0, y1, z1], [x0, y0, z1]],
        [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
    ];
    out.extend(faces);
}

/// A hand-written text `.x`: an L-shaped solid (asymmetric, so mirrored headings differ).
pub(super) fn block_x() -> String {
    let mut q = Vec::new();
    quads([-6.0, 0.0, -4.0], [6.0, 5.0, 4.0], &mut q);
    quads([2.0, 0.0, 4.0], [6.0, 8.0, 12.0], &mut q);
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut faces: Vec<String> = Vec::new();
    for quad in &q {
        let base = verts.len();
        verts.extend(quad.iter().copied());
        faces.push(format!("4;{},{},{},{}", base, base + 1, base + 2, base + 3));
        faces.push(format!("4;{},{},{},{}", base + 3, base + 2, base + 1, base));
    }
    let list = |items: Vec<String>| items.join(",\r\n").to_string() + ";";
    let v = list(
        verts
            .iter()
            .map(|p| format!("{:.3};{:.3};{:.3};", p[0], p[1], p[2]))
            .collect(),
    );
    let uv = list(
        (0..verts.len())
            .map(|i| format!("{}.0;0.0;", i % 2))
            .collect(),
    );
    let ids = vec!["0"; faces.len()].join(",");
    format!(
        "xof 0303txt 0032\r\nMesh Block {{\r\n{};\r\n{}\r\n{};\r\n{};\r\n\
         MeshMaterialList {{\r\n1;\r\n{};\r\n{};;\r\nMaterial M {{\r\n1.0;1.0;1.0;1.0;;\r\n\
         3.2;\r\n0.0;0.0;0.0;;\r\n0.0;0.0;0.0;;\r\nTextureFilename {{ \"block.bmp\"; }}\r\n}}\r\n}}\r\n\
         MeshTextureCoords {{\r\n{};\r\n{}\r\n}}\r\n}}\r\n",
        verts.len(),
        v,
        faces.len(),
        list(faces.clone()),
        faces.len(),
        ids,
        verts.len(),
        uv,
    )
}
