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
