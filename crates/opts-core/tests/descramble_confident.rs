//! `decode_confident` recovers a synthetic scrambled mesh (a bumpy grid with texture jitter).
use glam::{Vec2, Vec3};
use opts_core::formats::o3d::descramble::decode_confident;
use opts_core::formats::o3d::{vertex::Vertex, O3d};

/// Applies one of the four observed per-vertex permutations (self-inverse).
fn scramble(p: Vec3, class: u32) -> Vec3 {
    match class % 4 {
        0 => p,
        1 => Vec3::new(p.z, p.y, p.x),
        2 => Vec3::new(p.y, p.x, p.z),
        _ => Vec3::new(p.x, p.z, p.y),
    }
}

fn grid(n: usize) -> (O3d, Vec<Vec3>) {
    let mut truth = Vec::new();
    let mut vertices = Vec::new();
    let mut state = 12345u32;
    let mut rnd = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    for j in 0..n {
        for i in 0..n {
            // A gently curved sheet 20 m × 20 m, spanning distinct values on every axis.
            let (x, z) = (i as f32 * 1.25 - 12.0, j as f32 * 1.0 + 3.0);
            let p = Vec3::new(x, 0.3 + 0.02 * (x * 0.7).sin() + 0.01 * z, z);
            truth.push(p);
            vertices.push(Vertex {
                position: scramble(p, rnd() % 4),
                normal: Vec3::Y,
                uv: Vec2::new(i as f32 / n as f32, j as f32 / n as f32),
                color: None,
            });
        }
    }
    let mut indices = Vec::new();
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let a = (j * n + i) as u32;
            let (b, c, d) = (a + 1, a + n as u32, a + n as u32 + 1);
            indices.push([a, b, c]);
            indices.push([b, d, c]);
        }
    }
    let faces = indices.len();
    (
        O3d {
            version: 7,
            header_flags: 0,
            vertices,
            indices,
            face_materials: vec![0; faces],
            materials: Vec::new(),
            transform: None,
            encryption_key: Some(0x1234),
            descrambled: false,
            unknown_chunks: Vec::new(),
        },
        truth,
    )
}

#[test]
fn recovers_most_vertices_of_a_scrambled_sheet() {
    let (mut o, truth) = grid(14);
    let report = decode_confident(&mut o);
    assert!(report.accepted, "report: {report:?}");
    assert!(o.descrambled);
    let right = o
        .vertices
        .iter()
        .zip(&truth)
        .filter(|(v, t)| v.position.distance(**t) < 1e-4)
        .count();
    assert!(
        right as f32 / truth.len() as f32 > 0.97,
        "{right}/{}",
        truth.len()
    );
}

#[test]
fn rejects_a_mesh_it_cannot_decide() {
    // Two vertices and no faces carry no evidence.
    let (mut o, _) = grid(14);
    o.indices.clear();
    o.face_materials.clear();
    let before = o.clone();
    assert!(!decode_confident(&mut o).accepted);
    assert_eq!(o, before);
}
