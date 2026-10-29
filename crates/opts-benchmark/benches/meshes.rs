//! Criterion benchmark of the `.o3d` mesh parser (M-0028), on synthetic input written for this
//! file: the plain version-1 layout (header, vertex chunk, face chunk, one material, matrix) with a
//! chosen number of vertices and faces. No OMSI asset is needed.
//!
//! Run with `cargo bench -p opts-benchmark --no-default-features --bench meshes`.
//!
//! **What it does not measure:** most installed meshes are newer versions, and many are keyed
//! (scrambled) and take the exact decoder; this is the plain path, a lower bound for the cost of
//! reading a mesh, not the cost of a real vehicle or building.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use opts_core::formats::o3d::parse;

/// A version-1 `.o3d` with `vertices` vertices (32 bytes each) and `faces` faces (8 bytes each).
fn o3d_bytes(vertices: u16, faces: u16) -> Vec<u8> {
    let mut b = vec![0x84, 0x19, 0x01];
    b.push(0x17);
    b.extend_from_slice(&vertices.to_le_bytes());
    for i in 0..vertices {
        let t = f32::from(i) * 0.01;
        for f in [t, t * 0.5, 0.0, 0.0, 1.0, 0.0, t.fract(), (t * 0.5).fract()] {
            b.extend_from_slice(&f.to_le_bytes());
        }
    }
    b.push(0x49);
    b.extend_from_slice(&faces.to_le_bytes());
    for i in 0..faces {
        let a = i % vertices;
        for v in [a, (a + 1) % vertices, (a + 2) % vertices, 0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    b.push(0x26);
    b.extend_from_slice(&1u16.to_le_bytes());
    for f in [0.8f32, 0.8, 0.8, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 96.0] {
        b.extend_from_slice(&f.to_le_bytes());
    }
    b.push(8);
    b.extend_from_slice(b"test.bmp");
    b.push(0x79);
    for f in [
        1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ] {
        b.extend_from_slice(&f.to_le_bytes());
    }
    b
}

fn meshes(c: &mut Criterion) {
    // The generator is checked once: it must give what the parser reads.
    let sample = parse(&o3d_bytes(300, 600)).expect("the generated mesh parses");
    assert_eq!((sample.vertices.len(), sample.indices.len()), (300, 600));

    let mut group = c.benchmark_group("o3d_plain_v1");
    for (name, vertices, faces) in [
        ("small_1k_faces", 600, 1_000),
        ("building_10k_faces", 6_000, 10_000),
        ("vehicle_40k_faces", 24_000, 40_000),
    ] {
        let bytes = o3d_bytes(vertices, faces);
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_function(name, |b| {
            b.iter(|| black_box(parse(black_box(&bytes)).unwrap()));
        });
    }
    group.finish();
}

criterion_group!(benches, meshes);
criterion_main!(benches);
