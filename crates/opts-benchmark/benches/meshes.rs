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
use opts_core::formats::o3d::{parse, O3d};

#[path = "meshes/fixtures.rs"]
mod fixtures;
use fixtures::{keyed, plain, read, V1, V7, V7_WIDE};

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
