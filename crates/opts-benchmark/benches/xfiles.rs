//! Criterion benchmark of the DirectX `.x` mesh parser (M-0028), on synthetic text files written
//! for this file in the layout of the format page's example (`formats/binary/x.md`: a `Frame` with
//! a transform and one `Mesh` with normals, a material list and texture coordinates). No OMSI
//! asset is needed.
//!
//! Run with `cargo bench -p opts-benchmark --no-default-features --bench xfiles`. Only the text
//! form is measured: 8,119 of the 8,145 installed `.x` files are text, the 26 binary ones use an
//! unspecified variant (Q-X03). The generator is checked once per run.

use std::fmt::Write;
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use opts_core::formats::x::parse_x;

/// Writes `count` rows with `row(i)`, separated by `,` and closed by `;` (the array layout).
fn rows(s: &mut String, count: u32, row: impl Fn(u32) -> String) {
    for i in 0..count {
        let end = if i + 1 == count { ";" } else { "," };
        writeln!(s, "    {}{end}", row(i)).unwrap();
    }
}

/// A text `.x` with one mesh of `vertices` vertices and `faces` triangles, a normal and texture
/// coordinate per vertex and one textured material.
fn x_text(vertices: u32, faces: u32) -> String {
    let mut s =
        String::from("xof 0303txt 0032\r\n\r\nFrame Root {\r\n  FrameTransformMatrix {\r\n");
    s.push_str("    1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0;;\r\n  }\r\n");
    writeln!(s, "  Mesh Body {{\r\n    {vertices};").unwrap();
    rows(&mut s, vertices, |i| {
        let t = i as f32;
        format!(
            "{:.6};{:.6};{:.6};",
            (t * 0.37).sin() * 5.3,
            (t * 0.11).cos() * 2.7,
            t * 0.013
        )
    });
    let face = |i: u32| {
        let a = i % vertices;
        format!("3;{a},{},{};", (a + 1) % vertices, (a + 2) % vertices)
    };
    writeln!(s, "    {faces};").unwrap();
    rows(&mut s, faces, face);
    writeln!(s, "    MeshNormals {{\r\n    {vertices};").unwrap();
    rows(&mut s, vertices, |_| {
        "0.000000;1.000000;0.000000;".to_string()
    });
    writeln!(s, "    {faces};").unwrap();
    rows(&mut s, faces, face);
    s.push_str("    }\r\n    MeshMaterialList {\r\n    1;\r\n");
    writeln!(s, "    {faces};").unwrap();
    rows(&mut s, faces, |_| "0".to_string());
    s.push_str("      Material M {\r\n        1.0;1.0;1.0;1.0;;\r\n        3.2;\r\n        0.0;0.0;0.0;;\r\n");
    s.push_str("        0.0;0.0;0.0;;\r\n        TextureFilename { \"test.bmp\"; }\r\n      }\r\n    }\r\n");
    writeln!(s, "    MeshTextureCoords {{\r\n    {vertices};").unwrap();
    rows(&mut s, vertices, |i| {
        let t = i as f32;
        format!("{:.6};{:.6};", (t * 0.01).fract(), (t * 0.005).fract())
    });
    s.push_str("    }\r\n  }\r\n}\r\n");
    s
}

fn xfiles(c: &mut Criterion) {
    // The generator is checked once: it must give what the parser reads.
    let sample = parse_x(x_text(300, 600).as_bytes()).expect("the generated file parses");
    let mesh = sample.primary_mesh().expect("one mesh");
    assert_eq!((mesh.vertices.len(), mesh.indices.len()), (300, 600));
    assert_eq!(mesh.materials.len(), 1);

    let mut group = c.benchmark_group("x_text");
    for (name, vertices, faces) in [
        ("small_1k_faces", 600, 1_000),
        ("building_10k_faces", 6_000, 10_000),
    ] {
        let text = x_text(vertices, faces);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(name, |b| {
            b.iter(|| black_box(parse_x(black_box(text.as_bytes())).unwrap()));
        });
    }
    group.finish();
}

criterion_group!(benches, xfiles);
criterion_main!(benches);
