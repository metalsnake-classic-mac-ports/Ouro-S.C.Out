//! Sampled sounds.

#[derive(Debug, Clone)]
pub struct Sound {
    /// The number of the sound (the game uses 100–124, see
    /// `scout_core::sound::id`).
    pub id: i16,
    /// Samples per second: 11127 for the effects, 5564 for the sounds after
    /// a level (a half and a quarter of the Mac's 22254.5 Hz).
    pub rate: u32,
    /// 8-bit unsigned samples, 0x80 is silence.
    pub samples: Vec<u8>,
}

/// The music: a recording, played in a loop.
#[derive(Debug, Clone)]
pub struct Music {
    pub rate: u32,
    /// 16-bit mono samples.
    pub samples: Vec<i16>,
}
