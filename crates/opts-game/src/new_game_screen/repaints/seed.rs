//! Stable hashes that seed the per-instance repaint choice.

/// A well-mixed 64-bit hash of `parts` (SplitMix64 steps), stable across runs and platforms:
/// the seed of a per-instance repaint choice.
#[must_use]
pub fn instance_seed(parts: &[u64]) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
    for &p in parts {
        h ^= p;
        h = h.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = h;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        h = z ^ (z >> 31);
    }
    h
}

/// Stable 64-bit hash of a string (FNV-1a), for seeds made from names.
#[must_use]
pub fn name_seed(name: &str) -> u64 {
    name.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
    })
}
