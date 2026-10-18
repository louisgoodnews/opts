//! Exact decoding of keyed (scrambled) `.o3d` meshes.
//!
//! The exporter's scrambler is a small deterministic generator whose state can be followed from the
//! file header alone (`docs/src/formats/binary/o3d.md`, Addendum 14):
//!
//! * `s₀ = key + version − 4` (`+ 381` with the alternative-seed flag, header flags bit 1);
//! * for vertex `i`: `q = n·sᵢ mod 8000` with `n` the vertex count;
//! * `q` selects the position permutation (`q < 1000` swap x/y, `< 3000` swap x/z, `≤ 7000`
//!   identity, else swap y/z), the texture jitter (`u += ((q mod 100)/100)²` when `q mod 5 = 0`,
//!   `v += ((2q mod 100)/100)²` when `q mod 3 = 0`), the sign flips of the stored normal (slot x
//!   when `q mod 4 = 0`, y when `q mod 6 = 0`, z when `q mod 7 = 0`) and the normal's permutation;
//! * `sᵢ₊₁ = n·sᵢ + ⌊600·|frac(x)·frac(y)·frac(z)|⌋ mod 256` from the vertex's true position.
//!
//! Files with key `0` start at `s₀ = 0` and keep no memory: `sᵢ₊₁` is the data term alone; with
//! the alternative-seed flag the constant `KEY0_ALT_CONSTANT` (304) is added to it at every step.
//!
//! The jitter is `fl32(c²) / 10000` computed in float32 (`c = q mod 100`, or `2q mod 100` for v)
//! and was added in float32: `stored = fl32(u + j)`. Measured on the twins, that addition
//! reproduces the stored bits of 99.3 % of the jittered coordinates (the rest are plain copies
//! with other texture coordinates); `(c/100)²` reproduces only 79 %. The addition is not
//! injective, so `stored − j` is the original for 87 % of the jittered coordinates and one ulp off
//! for the rest; no rule picks the right neighbour better than the plain difference.
//!
//! Verified on the install's plain/keyed twins: positions exact for all 5,460 meshes with a key
//! and 934 of 957 with key `0` (`exact_twin_eval`).

use glam::{Vec2, Vec3};

use super::O3d;

/// Offset added to the seed of files written with the alternative seed (header flags bit 1).
const ALT_SEED_OFFSET: i64 = 381;
/// Constant added at every step by files with key `0` and the alternative-seed flag. Found by
/// scanning the constant of the memoryless recurrence over the 20 flagged key-`0` files of the
/// installation (Trabant, MB T1, Peugeot 106, the EN92/GN92 wagons): exactly one value decodes
/// every one of them to a coherent mesh (`o3d.md` Addendum 16).
const KEY0_ALT_CONSTANT: i64 = 304;
/// Range of the per-vertex state.
const MODULUS: i64 = 8000;
/// Below this `q` the normal of a swap-x/y vertex is stored with y and z swapped.
const NORMAL_YZ_BELOW: i64 = 600;
/// Above this `q` the normal of an identity vertex is stored with x and y swapped.
const NORMAL_XY_ABOVE: i64 = 4500;

/// What the generator decided for one vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexDecision {
    /// The state value `q` of the vertex.
    pub q: u32,
    /// Position permutation: `true[k] = stored[P[k]]`.
    pub position: [usize; 3],
    /// Normal permutation: `true[k] = unflipped[P[k]]`.
    pub normal: [usize; 3],
    /// Stored normal slots whose sign was flipped (x, y, z).
    pub flips: [bool; 3],
    /// Jitter added to the stored texture coordinates.
    pub jitter: Vec2,
}

/// The generator of a keyed mesh: its seed and whether the state accumulates (key `0` files keep
/// no memory). `None` for plain files.
#[must_use]
pub fn generator(o: &O3d) -> Option<(i64, bool)> {
    let key = o.encryption_key?;
    if key == 0 {
        return Some((0, false));
    }
    let mut k = i64::from(key) + i64::from(o.version) - 4;
    if o.header_flags & 2 != 0 {
        k += ALT_SEED_OFFSET;
    }
    Some((k.rem_euclid(MODULUS), true))
}

/// The constant added to the state at every step: `KEY0_ALT_CONSTANT` for files with key `0`
/// and the alternative-seed flag, otherwise zero.
#[must_use]
pub fn constant(o: &O3d) -> i64 {
    if o.encryption_key == Some(0) && o.header_flags & 2 != 0 {
        KEY0_ALT_CONSTANT
    } else {
        0
    }
}

/// The seed of a keyed mesh, or `None` for plain files.
#[must_use]
pub fn seed(o: &O3d) -> Option<i64> {
    generator(o).map(|(s, _)| s)
}

/// The decision for state value `q`.
#[must_use]
pub fn decision(q: i64) -> VertexDecision {
    let position = if q < 1000 {
        [1, 0, 2]
    } else if q < 3000 {
        [2, 1, 0]
    } else if q <= 7000 {
        [0, 1, 2]
    } else {
        [0, 2, 1]
    };
    let normal = if q < NORMAL_YZ_BELOW {
        [0, 2, 1]
    } else if q <= NORMAL_XY_ABOVE {
        [0, 1, 2]
    } else {
        [1, 0, 2]
    };
    let flips = [q % 4 == 0, q % 6 == 0, q % 7 == 0];
    let ju = if q % 5 == 0 { jitter(q % 100) } else { 0.0 };
    let jv = if q % 3 == 0 {
        jitter((2 * q) % 100)
    } else {
        0.0
    };
    VertexDecision {
        q: q as u32,
        position,
        normal,
        flips,
        jitter: Vec2::new(ju, jv),
    }
}

/// The texture jitter of code `c` (`0..100`), as the exporter computes it in float32.
fn jitter(c: i64) -> f32 {
    (c * c) as f32 / 10000.0
}

/// The state's data term for a true position.
#[must_use]
pub fn data_term(p: Vec3) -> i64 {
    let frac = |c: f32| f64::from(c) - f64::from(c).trunc();
    let product = (frac(p.x) * frac(p.y) * frac(p.z)).abs();
    ((600.0 * product).trunc() as i64) & 0xFF
}

fn permute(p: [usize; 3], v: Vec3) -> Vec3 {
    let a = [v.x, v.y, v.z];
    Vec3::new(a[p[0]], a[p[1]], a[p[2]])
}

/// Decodes the mesh in place: positions, texture coordinates and normals become the originals.
/// Returns the per-vertex decisions, or `None` when the file has no usable seed (plain, key `0`).
pub fn decode(o: &mut O3d) -> Option<Vec<VertexDecision>> {
    let (s, accumulate) = generator(o)?;
    let constant = constant(o);
    Some(decode_with(o, s, accumulate, constant))
}

/// Decodes the mesh in place with a given seed and state rule (the generator of a file is
/// [`generator`] and [`constant`]; tools try others on files whose generator is not known).
pub fn decode_with(o: &mut O3d, seed: i64, accumulate: bool, constant: i64) -> Vec<VertexDecision> {
    let mut s = seed;
    let n = o.vertices.len() as i64;
    let mut out = Vec::with_capacity(o.vertices.len());
    for v in &mut o.vertices {
        let q = (n * s).rem_euclid(MODULUS);
        let d = decision(q);
        v.position = permute(d.position, v.position);
        v.uv -= d.jitter;
        let mut nrm = v.normal;
        if d.flips[0] {
            nrm.x = -nrm.x;
        }
        if d.flips[1] {
            nrm.y = -nrm.y;
        }
        if d.flips[2] {
            nrm.z = -nrm.z;
        }
        v.normal = permute(d.normal, nrm);
        let carry = if accumulate { n * s } else { 0 };
        s = (carry + constant + data_term(v.position)).rem_euclid(MODULUS);
        out.push(d);
    }
    o.exact_decoded = true;
    out
}

/// The exporter's scrambling of a plain mesh (the inverse of [`decode`]), for tests and tools.
pub fn scramble(o: &mut O3d, key: u32) {
    o.encryption_key = Some(key);
    let (mut s, accumulate) = generator(o).expect("keyed");
    let constant = constant(o);
    let n = o.vertices.len() as i64;
    for v in &mut o.vertices {
        let q = (n * s).rem_euclid(MODULUS);
        let d = decision(q);
        let term = data_term(v.position);
        // true[k] = stored[P[k]]  <=>  stored[P[k]] = true[k]
        let mut stored = [0.0f32; 3];
        let t = [v.position.x, v.position.y, v.position.z];
        for k in 0..3 {
            stored[d.position[k]] = t[k];
        }
        v.position = Vec3::from(stored);
        v.uv += d.jitter;
        let mut nrm = [0.0f32; 3];
        let tn = [v.normal.x, v.normal.y, v.normal.z];
        for k in 0..3 {
            nrm[d.normal[k]] = tn[k];
        }
        for (k, flip) in d.flips.iter().enumerate() {
            if *flip {
                nrm[k] = -nrm[k];
            }
        }
        v.normal = Vec3::from(nrm);
        let carry = if accumulate { n * s } else { 0 };
        s = (carry + constant + term).rem_euclid(MODULUS);
    }
    o.exact_decoded = false;
}

#[cfg(test)]
mod tests {
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
}
