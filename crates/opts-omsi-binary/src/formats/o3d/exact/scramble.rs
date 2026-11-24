//! The exporter's scrambler run in reverse: a plain mesh is scrambled the way the keyed files are.

use glam::Vec3;

use super::{constant, data_term, decision, generator, MODULUS};
use crate::formats::o3d::O3d;

/// The exporter's scrambling of a plain mesh (the inverse of [`super::decode`]), for tests and tools.
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
