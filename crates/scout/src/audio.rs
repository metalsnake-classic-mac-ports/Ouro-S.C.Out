//! Sound output like the original's single channel: a new sound stops the
//! one playing. The Mac played at 22254.5 Hz; the effects (11127 Hz) and the
//! sounds after a level (5564 Hz) are repeated sample by sample up to that.

use anyhow::Result;
use scout_data::Sound;
use sdl3::audio::{AudioFormat, AudioSpec, AudioStreamOwner};

const OUTPUT_RATE: u32 = 22254;
/// Frames the audio device buffers: 256 (about 6 ms) instead of SDL's
/// default 1024 (23 ms), so sounds start with the picture.
const DEVICE_FRAMES: &str = "256";
const SILENCE: u8 = 0x80;

pub struct Audio {
    stream: Option<AudioStreamOwner>,
}

impl Audio {
    /// Open the default output; without one the game is silent. Call
    /// before anything else opens audio (the hint applies when the device
    /// opens).
    pub fn open(sdl: &sdl3::Sdl) -> Self {
        sdl3::hint::set("SDL_AUDIO_DEVICE_SAMPLE_FRAMES", DEVICE_FRAMES);
        let stream = (|| -> Result<AudioStreamOwner> {
            let spec = AudioSpec {
                freq: Some(OUTPUT_RATE as i32),
                channels: Some(1),
                format: Some(AudioFormat::U8),
            };
            let stream = sdl
                .audio()?
                .default_playback_device()
                .open_device_stream(Some(&spec))?;
            stream.resume()?;
            Ok(stream)
        })();
        match stream {
            Ok(stream) => Self {
                stream: Some(stream),
            },
            Err(e) => {
                eprintln!("no sound: {e}");
                Self { stream: None }
            }
        }
    }

    pub fn is_on(&self) -> bool {
        self.stream.is_some()
    }

    pub fn play(&self, sound: &Sound) {
        let Some(stream) = &self.stream else { return };
        let repeat = (OUTPUT_RATE as f32 / sound.rate as f32).round().max(1.0) as usize;
        let samples: Vec<u8> = sound
            .samples
            .iter()
            .flat_map(|&s| std::iter::repeat_n(s, repeat))
            .chain(std::iter::once(SILENCE))
            .collect();
        let _ = stream.clear();
        let _ = stream.put_data(&samples);
        // Without this the stream keeps the last samples back for its
        // resampler and never becomes empty, so the channel looks busy.
        let _ = stream.flush();
    }

    /// The channel is still playing.
    pub fn busy(&self) -> bool {
        self.stream
            .as_ref()
            .is_some_and(|s| s.queued_bytes().unwrap_or(0) > 0)
    }
}
