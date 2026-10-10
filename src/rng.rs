use serde::{Deserialize, Serialize};

/// MSVC CRT rand: explicit per-battle state, never the platform's random source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrtRng {
    pub state: u32,
    pub draws: u64,
}
impl CrtRng {
    pub const fn new(seed: u32) -> Self {
        Self {
            state: seed,
            draws: 0,
        }
    }
    pub fn next_u15(&mut self) -> u16 {
        self.state = self.state.wrapping_mul(214_013).wrapping_add(2_531_011);
        self.draws = self.draws.wrapping_add(1);
        ((self.state >> 16) & 0x7fff) as u16
    }
    /// Advance exactly `count` CRT calls in O(log count), preserving the full
    /// 32-bit state (not the truncated 15-bit value returned by rand()).
    pub fn advance(&mut self, count: u64) {
        let (mut mul, mut add) = (1u32, 0u32);
        let (mut step_mul, mut step_add) = (214_013u32, 2_531_011u32);
        let mut remaining = count;
        while remaining != 0 {
            if remaining & 1 != 0 {
                mul = mul.wrapping_mul(step_mul);
                add = add.wrapping_mul(step_mul).wrapping_add(step_add);
            }
            step_add = step_mul.wrapping_add(1).wrapping_mul(step_add);
            step_mul = step_mul.wrapping_mul(step_mul);
            remaining >>= 1;
        }
        self.state = self.state.wrapping_mul(mul).wrapping_add(add);
        self.draws = self.draws.wrapping_add(count);
    }
    /// Exact forward distance on the full-period 32-bit CRT cycle, in 32 steps.
    pub fn distance_to(self, target:u32)->u64 {
        let (mut state,mut mul,mut add,mut distance)=(self.state,214_013u32,2_531_011u32,0u64);
        for bit in 0..32 {
            if (state^target)&(1u32<<bit)!=0 {
                state=state.wrapping_mul(mul).wrapping_add(add);
                distance|=1u64<<bit;
            }
            add=add.wrapping_mul(mul.wrapping_add(1));
            mul=mul.wrapping_mul(mul);
        }
        debug_assert_eq!(state,target);
        distance
    }
    /// Source helper multiplies by this binary32 literal; do not replace with
    /// host doubles, modulo selection, or a different random distribution.
    pub fn unit(&mut self) -> f32 {
        f32::from(self.next_u15()) * 3.051_851e-5_f32
    }
    pub fn weighted_bool(&mut self, weight: f32) -> bool {
        if weight <= 0.0 {
            return false;
        }
        self.unit() <= weight // 100% still consumes a draw.
    }
    pub fn index(&mut self, len: u32) -> u32 {
        if len == 0 {
            return 0;
        }
        ((self.unit() * len as f32) as u32).min(len - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_distance_matches_jump_including_wraparound() {
        for seed in [0,1,1775700753,u32::MAX] {
            for steps in [0,1,2,17,1200,19290035,u32::MAX as u64,1u64<<32] {
                let from=CrtRng::new(seed);let mut to=from;to.advance(steps);
                assert_eq!(from.distance_to(to.state),steps% (1u64<<32));
            }
        }
    }
    #[test]
    fn jump_matches_individual_crt_calls() {
        for seed in [0, 1, 123, u32::MAX] {
            for count in [0, 1, 2, 3, 31, 1024, 65_536] {
                let mut reference = CrtRng::new(seed);
                // Include an existing draw count; advancement composes with next.
                reference.next_u15();
                let mut jumped = reference;
                for _ in 0..count {
                    reference.next_u15();
                }
                jumped.advance(count);
                assert_eq!(jumped, reference, "seed {seed}, advances {count}");
            }
        }
    }

    #[test]
    fn large_jumps_preserve_period_and_wrapping_draw_counter() {
        let mut rng = CrtRng::new(123);
        rng.advance(1u64 << 32);
        assert_eq!(rng.state, 123); // Full-period 32-bit CRT recurrence.
        assert_eq!(rng.draws, 1u64 << 32);
        let mut full = CrtRng::new(u32::MAX);
        full.advance(u64::MAX);
        let mut split = CrtRng::new(u32::MAX);
        split.advance(1u64 << 63);
        split.advance((1u64 << 63) - 1);
        assert_eq!(full, split);
        full.next_u15();
        assert_eq!(full, CrtRng::new(u32::MAX));
    }
}
