//! Unit tests for `super` (`exact.rs`).

use super::*;
use crate::formats::o3d::Vertex;

fn mesh(n: usize, key: Option<u32>) -> O3d {
    let vertices = (0..n)
        .map(|i| {
            let t = i as f32 * 0.731;
            Vertex {
                position: Vec3::new(t.sin() * 3.0 + 1.37, t.cos() * 2.0 - 0.21, t * 0.5 - 1.1),
                normal: Vec3::new(0.3, -0.5, 0.8).normalize(),
                uv: Vec2::new(0.25 + 0.01 * i as f32, 0.75 - 0.02 * i as f32),
                color: None,
            }
        })
        .collect();
    O3d {
        version: 7,
        header_flags: 0,
        vertices,
        indices: vec![[0, 1, 2]],
        face_materials: vec![0],
        materials: Vec::new(),
        transform: None,
        encryption_key: key,
        exact_decoded: false,
        unknown_chunks: Vec::new(),
    }
}

/// A mesh whose stored normals disagree with its faces and whose positions have no floor: the
/// coherence heuristics of `vertex_layout` call it unreadable.
fn incoherent(key: Option<u32>) -> O3d {
    let mut m = mesh(60, key);
    for (i, v) in m.vertices.iter_mut().enumerate() {
        let a = i as f32 * 2.399;
        v.position = Vec3::new(a.sin() * 2.0, a.cos() * 2.0, (a * 0.37).sin() * 2.0);
        v.normal = if i % 2 == 0 { Vec3::X } else { -Vec3::X };
    }
    m.indices = (0..58).map(|i| [i, i + 1, i + 2]).collect();
    m.face_materials = vec![0; m.indices.len()];
    m
}

/// M-0066: an exactly decoded mesh is trusted whatever its normals look like, like a plain
/// file; the loader used to refuse such meshes (`Unreadable`), so objects were missing.
#[test]
fn an_exactly_decoded_mesh_is_never_unreadable() {
    let plain = incoherent(None);
    assert_eq!(
        plain.vertex_layout(),
        super::super::VertexLayout::RecomputedNormals,
        "the same mesh as a plain file"
    );
    let mut keyed = incoherent(None);
    scramble(&mut keyed, 0x31B6);
    assert_eq!(
        keyed.vertex_layout(),
        super::super::VertexLayout::Unreadable,
        "scrambled: unreadable until decoded"
    );
    decode(&mut keyed).expect("keyed");
    assert_eq!(
        keyed.vertex_layout(),
        super::super::VertexLayout::RecomputedNormals
    );
}

#[test]
fn scrambling_and_decoding_round_trip() {
    for (key, flags) in [(0x31B6, 0), (13730, 2), (0, 0), (5, 0), (0, 2)] {
        let mut plain = mesh(40, None);
        plain.header_flags = flags;
        let original = plain.clone();
        scramble(&mut plain, key);
        assert!(plain
            .vertices
            .iter()
            .zip(&original.vertices)
            .any(|(a, b)| a.position != b.position));
        let decisions = decode(&mut plain).expect("keyed");
        assert_eq!(decisions.len(), 40);
        for (a, b) in plain.vertices.iter().zip(&original.vertices) {
            assert_eq!(a.position, b.position);
            // The exporter's float32 addition is not injective: one ulp may be lost.
            assert!(
                (a.uv - b.uv).abs().max_element() <= 2.0 * f32::EPSILON,
                "{} {}",
                a.uv,
                b.uv
            );
            assert!((a.normal - b.normal).abs().max_element() < 1e-6);
        }
        assert!(plain.exact_decoded);
        assert_eq!(plain.vertex_layout(), super::super::VertexLayout::Standard);
    }
}

#[test]
fn classes_follow_the_boundaries() {
    assert_eq!(decision(0).position, [1, 0, 2]);
    assert_eq!(decision(999).position, [1, 0, 2]);
    assert_eq!(decision(1000).position, [2, 1, 0]);
    assert_eq!(decision(2999).position, [2, 1, 0]);
    assert_eq!(decision(3000).position, [0, 1, 2]);
    assert_eq!(decision(7000).position, [0, 1, 2]);
    assert_eq!(decision(7001).position, [0, 2, 1]);
    assert_eq!(decision(7999).position, [0, 2, 1]);
}

#[test]
fn normal_permutation_boundaries() {
    assert_eq!(decision(599).normal, [0, 2, 1]);
    assert_eq!(decision(600).normal, [0, 1, 2]);
    assert_eq!(decision(4500).normal, [0, 1, 2]);
    assert_eq!(decision(4501).normal, [1, 0, 2]);
    assert_eq!(decision(7999).normal, [1, 0, 2]);
}

#[test]
fn jitter_and_flips_follow_the_residues() {
    let d = decision(60);
    assert!((d.jitter.x - 0.36).abs() < 1e-6, "{}", d.jitter.x);
    assert!((d.jitter.y - 0.04).abs() < 1e-6, "{}", d.jitter.y);
    assert_eq!(d.flips, [true, true, false]);
    let d = decision(7);
    assert_eq!(d.jitter, Vec2::ZERO);
    assert_eq!(d.flips, [false, false, true]);
}

#[test]
fn the_data_term_is_a_byte_of_the_scaled_fraction_product() {
    assert_eq!(data_term(Vec3::new(0.5, 0.25, 0.0805)), 6);
    assert_eq!(data_term(Vec3::new(-0.375, 0.0, -0.0001)), 0);
    assert_eq!(data_term(Vec3::new(6.98448, 2.20462, -0.0954549)), 11);
    assert_eq!(data_term(Vec3::new(-0.9828, 2.9496, 4.4563)), 255);
}
