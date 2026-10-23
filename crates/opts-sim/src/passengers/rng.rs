//! The random source of the passenger simulation.

/// A small deterministic random source (SplitMix64).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A source with `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// A Poisson-distributed count with mean `mean` (Knuth's product for small means, a normal
    /// approximation above 30; never above `4 * mean + 20`).
    pub fn poisson(&mut self, mean: f32) -> u32 {
        if mean <= 0.0 {
            return 0;
        }
        if mean < 30.0 {
            let limit = (-mean).exp();
            let (mut k, mut p) = (0u32, 1.0f32);
            loop {
                p *= self.next_f32();
                if p <= limit {
                    return k;
                }
                k += 1;
            }
        }
        // Sum of twelve uniforms minus six: mean 0, variance 1.
        let z: f32 = (0..12).map(|_| self.next_f32()).sum::<f32>() - 6.0;
        (mean + mean.sqrt() * z)
            .round()
            .clamp(0.0, 4.0 * mean + 20.0) as u32
    }

    /// The next value in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x >> 40) as f32 / 16_777_216.0
    }
}
