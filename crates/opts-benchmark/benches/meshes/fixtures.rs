//! Synthetic `.o3d` inputs of the mesh benchmark: the plain layouts and the keyed version-7 mesh,
//! written in the format's layouts and read back the way the loader does.

use opts_core::formats::o3d::{exact, parse, O3d};
/// A key as found in installed meshes (format page Addendum 4).
pub(super) const KEY: u32 = 0x31B6;

/// The layout of one generated file.
#[derive(Clone, Copy)]
pub(super) struct Layout {
    pub(super) version: u8,
    /// Flags byte (version 3 and later); bit 0 selects 32-bit face indices.
    pub(super) flags: u8,
}

pub(super) const V1: Layout = Layout {
    version: 1,
    flags: 0,
};
pub(super) const V7: Layout = Layout {
    version: 7,
    flags: 0,
};
pub(super) const V7_WIDE: Layout = Layout {
    version: 7,
    flags: 1,
};

/// Vertex `i` of a mesh: position, normal and texture coordinates with varied fractions (the
/// keyed generator's data term reads the fractional parts of the position).
fn vertex(i: u32) -> [f32; 8] {
    let t = i as f32;
    let (x, y, z) = ((t * 0.37).sin() * 5.3, (t * 0.11).cos() * 2.7, t * 0.013);
    [
        x,
        y,
        z,
        0.0,
        1.0,
        0.0,
        (t * 0.01).fract(),
        (t * 0.005).fract(),
    ]
}

fn count(b: &mut Vec<u8>, layout: Layout, n: u32) {
    if layout.version == 1 {
        b.extend_from_slice(&(n as u16).to_le_bytes());
    } else {
        b.extend_from_slice(&n.to_le_bytes());
    }
}

/// A `.o3d` file of `layout` with the given vertices, `faces` faces and one material; `key`
/// goes into the header of version 4 and later (`None` writes the plain marker `0xFFFF_FFFF`).
fn o3d_bytes(layout: Layout, vertices: &[[f32; 8]], faces: u32, key: Option<u32>) -> Vec<u8> {
    let mut b = vec![0x84, 0x19, layout.version];
    if layout.version >= 3 {
        b.push(layout.flags);
    }
    if layout.version >= 4 {
        b.extend_from_slice(&key.unwrap_or(u32::MAX).to_le_bytes());
    }
    b.push(0x17);
    count(&mut b, layout, vertices.len() as u32);
    for v in vertices {
        v.iter().for_each(|f| b.extend_from_slice(&f.to_le_bytes()));
    }
    b.push(0x49);
    count(&mut b, layout, faces);
    let n = vertices.len() as u32;
    for i in 0..faces {
        let a = i % n;
        for v in [a, (a + 1) % n, (a + 2) % n] {
            if layout.flags & 1 == 1 {
                b.extend_from_slice(&v.to_le_bytes());
            } else {
                b.extend_from_slice(&(v as u16).to_le_bytes());
            }
        }
        b.extend_from_slice(&0u16.to_le_bytes());
    }
    b.push(0x26);
    b.extend_from_slice(&1u16.to_le_bytes());
    for f in [0.8f32, 0.8, 0.8, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 96.0] {
        b.extend_from_slice(&f.to_le_bytes());
    }
    b.push(8);
    b.extend_from_slice(b"test.bmp");
    b.push(0x79);
    for k in 0..16 {
        let f: f32 = if k % 5 == 0 { 1.0 } else { 0.0 };
        b.extend_from_slice(&f.to_le_bytes());
    }
    b
}

pub(super) fn plain(layout: Layout, vertices: u32, faces: u32) -> Vec<u8> {
    let vs: Vec<_> = (0..vertices).map(vertex).collect();
    o3d_bytes(layout, &vs, faces, None)
}

/// A keyed version-7 file: the plain mesh scrambled under [`KEY`] by the exporter model.
pub(super) fn keyed(vertices: u32, faces: u32) -> Vec<u8> {
    let mut mesh = parse(&plain(V7, vertices, faces)).expect("the plain mesh parses");
    exact::scramble(&mut mesh, KEY);
    let stored: Vec<_> = mesh
        .vertices
        .iter()
        .map(|v| {
            let (p, n) = (v.position, v.normal);
            [p.x, p.y, p.z, n.x, n.y, n.z, v.uv.x, v.uv.y]
        })
        .collect();
    o3d_bytes(V7, &stored, faces, Some(KEY))
}

/// Parse and, for a keyed mesh, decode: what the loader does with a file.
pub(super) fn read(bytes: &[u8]) -> O3d {
    let mut mesh = parse(bytes).unwrap();
    if mesh.encryption_key.is_some() {
        exact::decode(&mut mesh).expect("the generator covers the mesh");
    }
    mesh
}
