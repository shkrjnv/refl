//! The core PRNG, `SplitMix64`. Wherever the core needs a random choice, the
//! seed comes in as an argument and is written into the event or the model
//! parameters (`docs/01-architecture.md`, principle «Ядро детерминировано»).

/// `SplitMix64` (Steele, Lea, Flood 2014): one 64-bit word of state, integer
/// arithmetic only, so every platform yields the same sequence.
#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)` with 53 random bits.
    #[allow(clippy::cast_precision_loss)] // 53 bits fit an f64 mantissa exactly
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Standard normal by Box–Muller, one value per two uniforms, through
    /// `libm` so the result is bit-identical everywhere.
    pub fn next_normal(&mut self) -> f64 {
        let u1 = 1.0 - self.next_f64(); // (0, 1]: ln is finite
        let u2 = self.next_f64();
        libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(2.0 * core::f64::consts::PI * u2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_reference_sequence() {
        // First outputs for seed 0 from the reference C implementation.
        let mut r = SplitMix64::new(0);
        assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(r.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(r.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn uniforms_stay_in_range() {
        let mut r = SplitMix64::new(42);
        for _ in 0..10_000 {
            let u = r.next_f64();
            assert!((0.0..1.0).contains(&u));
            assert!(r.next_normal().is_finite());
        }
    }
}
