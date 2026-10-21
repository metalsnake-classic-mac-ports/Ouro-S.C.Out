//! Sound output like the original's single channel: a new sound stops the
//! one playing. The Mac played at 22254.5 Hz; the effects (11127 Hz) and the
//! sounds after a level (5564 Hz) are repeated sample by sample up to that.

use anyhow::Result;
use scout_core::music::{self, MusicPlayer};
use scout_data::{GameData, Sound};
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

/// Music kept queued ahead: enough for a late frame, little enough that
/// stopping is quick.
const MUSIC_AHEAD: f64 = 0.1;

/// The music on its own channel, as in the original (CODE 10 0x5e86 opens a
/// second sound channel and double buffer at 14162 Hz).
pub struct Music {
    stream: Option<AudioStreamOwner>,
    player: Option<MusicPlayer>,
    /// Stopped because the game went to the background (`-0x6db4`).
    suspended: bool,
}

impl Music {
    pub fn open(sdl: &sdl3::Sdl) -> Self {
        let stream = (|| -> Result<AudioStreamOwner> {
            let spec = AudioSpec {
                freq: Some(music::RATE.round() as i32),
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
        if let Err(e) = &stream {
            eprintln!("no music: {e}");
        }
        Self {
            stream: stream.ok(),
            player: None,
            suspended: false,
        }
    }

    /// Start the song at an order position (the game: 1 at launch, 0 when
    /// it comes back to the front).
    pub fn start(&mut self, data: &GameData, position: u8) {
        self.stop();
        self.player = Some(MusicPlayer::new(&data.music, &data.music_code, position));
    }

    /// The game goes to the background: the music stops (CODE 7 0x105a).
    pub fn suspend(&mut self) {
        if self.player.is_some() {
            self.stop();
            self.suspended = true;
        }
    }

    /// Back in front: the main loop starts the song again, from position 0
    /// (CODE 7 0x7d8).
    pub fn resume(&mut self, data: &GameData) {
        if self.suspended {
            self.suspended = false;
            self.start(data, 0);
        }
    }

    pub fn stop(&mut self) {
        self.player = None;
        if let Some(stream) = &self.stream {
            let _ = stream.clear();
        }
    }

    /// Mix ticks until the queue holds [`MUSIC_AHEAD`] seconds.
    pub fn fill(&mut self) {
        let (Some(stream), Some(player)) = (&self.stream, &mut self.player) else {
            return;
        };
        let ahead = (music::RATE * MUSIC_AHEAD) as i32;
        while stream.queued_bytes().unwrap_or(ahead) < ahead {
            if stream.put_data(&player.tick()).is_err() {
                return;
            }
        }
    }
}
