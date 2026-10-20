//! Sampled sounds: `'snd '` resources of format 1 with one `bufferCmd`
//! pointing to a standard sound header (Inside Macintosh: Sound).

use crate::bytes::{slice, u8_at, u16_at, u32_at};
use crate::error::{Result, corrupt};

/// `bufferCmd` with the "offset in this resource" flag.
const BUFFER_CMD: u16 = 0x8051;
/// Standard sound header encoding (8-bit unsigned samples).
const STANDARD_HEADER: u8 = 0;

#[derive(Debug, Clone)]
pub struct Sound {
    /// Resource ID (the game uses 100–124).
    pub id: i16,
    /// Samples per second, rounded: 11127 for the effects, 5564 for the
    /// sounds after a level (a half and a quarter of the Mac's 22254.5 Hz).
    pub rate: u32,
    /// 8-bit unsigned samples, 0x80 is silence.
    pub samples: Vec<u8>,
}

impl Sound {
    pub fn from_snd(id: i16, data: &[u8]) -> Result<Self> {
        let bad = |what: &str| corrupt(format!("'snd ' {id}: {what}"));
        if u16_at(data, 0)? != 1 {
            return Err(bad("not format 1"));
        }
        let modifiers = u16_at(data, 2)? as usize;
        let commands_at = 4 + 6 * modifiers;
        let commands = u16_at(data, commands_at)? as usize;
        let header = (0..commands)
            .map(|i| commands_at + 2 + 8 * i)
            .find_map(|at| match u16_at(data, at) {
                Ok(BUFFER_CMD) => Some(u32_at(data, at + 4).map(|o| o as usize)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .ok_or_else(|| bad("no bufferCmd"))??;
        let length = u32_at(data, header + 4)? as usize;
        let rate = (u32_at(data, header + 8)? + 0x8000) >> 16;
        if u8_at(data, header + 20)? != STANDARD_HEADER {
            return Err(bad("not a standard sound header"));
        }
        let samples = slice(data, header + 22, length)?.to_vec();
        Ok(Self { id, rate, samples })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_standard_sound() {
        let mut data = vec![0, 1, 0, 1, 0, 5, 0, 0, 0, 0xA0, 0, 1];
        data.extend([0x80, 0x51, 0, 0, 0, 0, 0, 20]);
        data.extend([0, 0, 0, 0, 0, 0, 0, 3, 0x2B, 0x77, 0x45, 0xD1]);
        data.extend([0; 8]);
        data.extend([0, 0x3C, 1, 2, 3]);
        let sound = Sound::from_snd(100, &data).unwrap();
        assert_eq!(sound.rate, 11127);
        assert_eq!(sound.samples, [1, 2, 3]);
    }
}
