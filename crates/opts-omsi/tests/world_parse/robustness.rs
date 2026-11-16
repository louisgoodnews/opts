//! Robustness of the world and scenery parsers: pseudo-random input must never panic.

use super::*;

#[test]
fn test_robustness_pseudorandom_fuzzing() {
    // Generate pseudo-random buffers and verify no panics occur
    let mut state: u64 = 0xDEADBEEFCAFEBABE;
    let mut lcg = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 32) as u8
    };

    for len in [0, 1, 3, 4, 19, 20, 21, 50, 100, 1000] {
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(lcg());
        }

        // None of these parsers should ever panic on garbage input
        let _ = SceneryObject::parse(&buf);
        let _ = SplineProfile::parse(&buf);
        let _ = MapManifest::parse(&buf);
        let _ = MapTile::parse(&buf);
        let _ = MapTerrain::parse(&buf);
        let _ = MapWater::parse(&buf);
    }
}
