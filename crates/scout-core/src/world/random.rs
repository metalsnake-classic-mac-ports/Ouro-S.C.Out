//! The Toolbox `Random` of the classic Mac OS, which the original uses for
//! homing projectiles, random diagonal shooters and screen transitions.
//!
//! `randSeed := randSeed × 16807 mod (2³¹ − 1)`; the result is the low word
//! as a signed number, with −32768 mapped to 0. `InitGraf` sets the seed
//! to 1 and the game never changes it. (Not yet checked number for number
//! against a real Mac.)

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacRandom {
    pub seed: u32,
}

impl Default for MacRandom {
    fn default() -> Self {
        Self { seed: 1 }
    }
}

impl MacRandom {
    pub fn random(&mut self) -> i16 {
        self.seed = ((self.seed as u64 * 16807) % 0x7FFF_FFFF) as u32;
        let value = self.seed as u16 as i16;
        if value == i16::MIN { 0 } else { value }
    }

    /// `Random() & 0x7fff`, as the game uses it.
    pub fn next_positive(&mut self) -> i32 {
        (self.random() as i32) & 0x7FFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn park_miller_sequence() {
        let mut r = MacRandom::default();
        r.random();
        assert_eq!(r.seed, 16807);
        r.random();
        assert_eq!(r.seed, 282_475_249);
        assert_eq!(MacRandom { seed: 1 }.random(), 16807);
    }
}
