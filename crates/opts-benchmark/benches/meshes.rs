//! Criterion benchmark of the `.o3d` mesh parser (M-0028), on synthetic input written for this
//! file in the layouts of the format page (`formats/binary/o3d.md`): the plain version-1 layout
//! (3-byte header, 16-bit counts), version 7 (8-byte header, 32-bit counts) with 16-bit and with
//! 32-bit face indices (flags bit 0), and keyed version-7 meshes. No OMSI asset is needed.
//!
//! Run with `cargo bench -p opts-benchmark --no-default-features --bench meshes`.
//!
//! A keyed mesh is written by the project's own exporter model (`o3d::exact::scramble`, the
//! inverse of the exact decoder, format page Addendum 14) and timed as the loader reads it: parse,
//! then `o3d::exact::decode`. The vertex data is synthetic, so the cost of the decoder's data term
//! follows these coordinates, not a real model's.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use opts_core::formats::o3d::{exact, parse, O3d};

/// A key as found in installed meshes (format page Addendum 4).
const KEY: u32 = 0x31B6;

/// The layout of one generated file.
#[derive(Clone, Copy)]
struct Layout {
    version: u8,
    /// Flags byte (version 3 and later); bit 0 selects 32-bit face indices.
    flags: u8,
}

const V1: Layout = Layout {
    version: 1,
    flags: 0,
};
const V7: Layout = Layout {
    version: 7,
    flags: 0,
};
const V7_WIDE: Layout = Layout {
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

fn plain(layout: Layout, vertices: u32, faces: u32) -> Vec<u8> {
    let vs: Vec<_> = (0..vertices).map(vertex).collect();
    o3d_bytes(layout, &vs, faces, None)
}

/// A keyed version-7 file: the plain mesh scrambled under [`KEY`] by the exporter model.
fn keyed(vertices: u32, faces: u32) -> Vec<u8> {
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
fn read(bytes: &[u8]) -> O3d {
    let mut mesh = parse(bytes).unwrap();
    if mesh.encryption_key.is_some() {
        exact::decode(&mut mesh).expect("the generator covers the mesh");
    }
    mesh
}

/// The generators are checked once: they must give what the parser reads, and a keyed mesh must
/// be scrambled and decode back to the plain positions.
fn check_generators() {
    for layout in [V1, V7, V7_WIDE] {
        let mesh = parse(&plain(layout, 300, 600)).expect("the generated mesh parses");
        assert_eq!((mesh.vertices.len(), mesh.indices.len()), (300, 600));
    }
    let original = parse(&plain(V7, 300, 600)).unwrap();
    let scrambled = parse(&keyed(300, 600)).unwrap();
    let same = |m: &O3d| {
        m.vertices
            .iter()
            .zip(&original.vertices)
            .all(|(a, b)| a.position == b.position)
    };
    assert!(!same(&scrambled), "the keyed mesh is scrambled");
    let decoded = read(&keyed(300, 600));
    assert!(
        decoded.exact_decoded && same(&decoded),
        "the keyed mesh decodes exactly"
    );
}

fn meshes(c: &mut Criterion) {
    check_generators();
    let cases: [(&str, &str, Vec<u8>); 8] = [
        ("o3d_plain_v1", "small_1k_faces", plain(V1, 600, 1_000)),
        (
            "o3d_plain_v1",
            "building_10k_faces",
            plain(V1, 6_000, 10_000),
        ),
        (
            "o3d_plain_v1",
            "vehicle_40k_faces",
            plain(V1, 24_000, 40_000),
        ),
        (
            "o3d_plain_v7",
            "building_10k_faces",
            plain(V7, 6_000, 10_000),
        ),
        (
            "o3d_plain_v7",
            "vehicle_40k_faces",
            plain(V7, 24_000, 40_000),
        ),
        (
            "o3d_plain_v7_32bit",
            "large_120k_faces",
            plain(V7_WIDE, 72_000, 120_000),
        ),
        ("o3d_keyed_v7", "building_10k_faces", keyed(6_000, 10_000)),
        ("o3d_keyed_v7", "vehicle_40k_faces", keyed(24_000, 40_000)),
    ];
    let mut groups: Vec<&str> = cases.iter().map(|c| c.0).collect();
    groups.dedup();
    for name in groups {
        let mut group = c.benchmark_group(name);
        for (_, case, bytes) in cases.iter().filter(|c| c.0 == name) {
            group.throughput(Throughput::Bytes(bytes.len() as u64));
            group.bench_function(*case, |b| b.iter(|| black_box(read(black_box(bytes)))));
        }
        group.finish();
    }
}

criterion_group!(benches, meshes);
criterion_main!(benches);
