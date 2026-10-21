//! Sound output like the original's single channel: a new sound stops the
//! one playing. The Mac played at 22254.5 Hz; the effects (11127 Hz) and the
//! sounds after a level (5564 Hz) are repeated sample by sample up to that.

use anyhow::Result;
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
/// second sound channel): the recording from the data pack in a loop.
pub struct Music {
    audio: Option<sdl3::AudioSubsystem>,
    stream: Option<AudioStreamOwner>,
    /// The song and where it is.
    song: Option<(Vec<i16>, usize)>,
    rate: u32,
    /// Stopped because the game went to the background (`-0x6db4`).
    suspended: bool,
}

impl Music {
    pub fn open(sdl: &sdl3::Sdl) -> Self {
        Self {
            audio: sdl.audio().ok(),
            stream: None,
            song: None,
            rate: 0,
            suspended: false,
        }
    }

    /// Start the song from the beginning. (The original starts at order
    /// position 1 when it launches; that is not to be heard.)
    pub fn start(&mut self, data: &GameData) {
        self.stop();
        if data.music.samples.is_empty() {
            return;
        }
        if self.stream.is_none() || self.rate != data.music.rate {
            self.rate = data.music.rate;
            self.stream = self.open_stream(data.music.rate);
        }
        self.song = Some((data.music.samples.clone(), 0));
    }

    fn open_stream(&self, rate: u32) -> Option<AudioStreamOwner> {
        let audio = self.audio.as_ref()?;
        let spec = AudioSpec {
            freq: Some(rate as i32),
            channels: Some(1),
            format: Some(AudioFormat::s16_sys()),
        };
        let stream = audio
            .default_playback_device()
            .open_device_stream(Some(&spec))
            .and_then(|s| s.resume().map(|_| s));
        match stream {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("no music: {e}");
                None
            }
        }
    }

    /// The game goes to the background: the music stops (CODE 7 0x105a).
    pub fn suspend(&mut self) {
        if let (Some(stream), Some(_)) = (&self.stream, &self.song) {
            let _ = stream.pause();
            self.suspended = true;
        }
    }

    /// Back in front: the music goes on where it stopped. (The original
    /// starts the song again, CODE 7 0x7d8.)
    pub fn resume(&mut self) {
        if let (Some(stream), true) = (&self.stream, self.suspended) {
            let _ = stream.resume();
            self.suspended = false;
        }
    }

    pub fn stop(&mut self) {
        self.song = None;
        if let Some(stream) = &self.stream {
            let _ = stream.clear();
            if self.suspended {
                let _ = stream.resume();
            }
        }
        self.suspended = false;
    }

    pub fn is_playing(&self) -> bool {
        self.song.is_some()
    }

    /// Queue the song until [`MUSIC_AHEAD`] seconds are waiting; at its end
    /// it starts again.
    pub fn fill(&mut self) {
        let (Some(stream), Some((song, at))) = (&self.stream, &mut self.song) else {
            return;
        };
        let ahead = (self.rate as f64 * MUSIC_AHEAD) as i32 * 2;
        while stream.queued_bytes().unwrap_or(ahead) < ahead {
            let end = (*at + 1024).min(song.len());
            if stream.put_data_i16(&song[*at..end]).is_err() {
                return;
            }
            *at = if end == song.len() { 0 } else { end };
        }
    }
}
