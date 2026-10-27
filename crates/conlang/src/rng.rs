//! Deterministic randomness: the same seed gives the same language on every platform.
//!
//! SplitMix64 for the stream and FNV-1a 64 for deriving sub-seeds from text, both written
//! from their public descriptions. No `rand` dependency, so a dependency update can never
//! change an existing language.

/// SplitMix64 generator.
#[derive(Debug, Clone)]
pub(crate) struct Rng {
    state: u64,
}

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// A generator for one named purpose ("phonology", a gloss, …) derived from `seed`.
    pub(crate) fn derive(seed: u64, purpose: &str, attempt: u32) -> Self {
        let mut h = fnv1a(purpose.as_bytes());
        h ^= u64::from(attempt).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        Self::new(seed ^ h.rotate_left(17))
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n`; 0 when `n` is 0.
    pub(crate) fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        // Lemire's multiply-shift; the tiny bias is irrelevant here and it never loops.
        ((u64::from(self.next_u64() as u32) * u64::from(n)) >> 32) as u32
    }

    /// Uniform in `lo..=hi` (either order).
    pub(crate) fn range(&mut self, lo: u8, hi: u8) -> u8 {
        let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        let span = u32::from(hi - lo) + 1;
        lo.saturating_add(self.below(span) as u8)
    }

    /// True with `percent` % probability (values above 100 count as 100).
    pub(crate) fn chance(&mut self, percent: u8) -> bool {
        self.below(100) < u32::from(percent.min(100))
    }

    /// Index chosen in proportion to `weights`; `None` when every weight is 0.
    pub(crate) fn weighted(&mut self, weights: &[u32]) -> Option<usize> {
        let total: u64 = weights.iter().map(|&w| u64::from(w)).sum();
        if total == 0 {
            return None;
        }
        let mut pick = self.next_u64() % total;
        for (i, &w) in weights.iter().enumerate() {
            let w = u64::from(w);
            if pick < w {
                return Some(i);
            }
            pick -= w;
        }
        None
    }
}

/// FNV-1a 64-bit hash.
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix_reference_values() {
        // First outputs of SplitMix64 seeded with 0 (published reference sequence).
        let mut r = Rng::new(0);
        assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(r.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    fn fnv_reference_values() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn helpers_stay_in_bounds() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            assert!(r.below(5) < 5);
            let v = r.range(3, 1);
            assert!((1..=3).contains(&v));
            assert!(r.range(255, 255) == 255);
        }
        assert_eq!(r.below(0), 0);
        assert_eq!(r.weighted(&[0, 0]), None);
        assert_eq!(r.weighted(&[0, 5, 0]), Some(1));
        assert_eq!(r.weighted(&[u32::MAX, u32::MAX]).map(|i| i < 2), Some(true));
    }
}
