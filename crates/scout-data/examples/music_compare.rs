//! Write the music as a stereo WAV to compare the players: left the
//! original (with its bugs), right the corrected one.
//!
//! Usage: cargo run --release -p scout-data --example music_compare -- [DISK] OUT.wav [SECONDS]

use std::io::Write;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
use scout_core::music::{MusicPlayer, RATE};
use scout_data::GameData;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (disk, rest) = match args.len() {
        0 => return Err("usage: music_compare [DISK] OUT.wav [SECONDS]".into()),
        1 => (None, &args[..]),
        _ if args[1].ends_with(".wav") => (Some(&args[0]), &args[1..]),
        _ => (None, &args[..]),
    };
    let disk = disk
        .cloned()
        .or_else(|| std::env::var("SCOUT_DISK").ok())
        .ok_or("no disk image: pass one or set SCOUT_DISK")?;
    let out = &rest[0];
    let seconds: f64 = rest.get(1).map_or(Ok(210.0), |s| s.parse())?;
    let data = GameData::load(std::path::Path::new(&disk))?;
    // the game starts the music at position 1
    let mut original = MusicPlayer::new(&data.music, &data.music_volumes, 1);
    let mut fixed = MusicPlayer::new(&data.music, &data.music_volumes, 1);
    fixed.fixed = true;
    let ticks = (seconds * RATE / scout_core::music::TICK_SAMPLES as f64) as usize;
    let mut samples = Vec::new();
    for _ in 0..ticks {
        for (l, r) in original.tick().into_iter().zip(fixed.tick()) {
            for s in [l, r] {
                samples.extend_from_slice(&((s as i16 - 128) << 8).to_le_bytes());
            }
        }
    }
    let rate = RATE.round() as u32;
    let mut f = std::io::BufWriter::new(std::fs::File::create(out)?);
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + samples.len() as u32).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?; // PCM
    f.write_all(&2u16.to_le_bytes())?; // stereo
    f.write_all(&rate.to_le_bytes())?;
    f.write_all(&(rate * 4).to_le_bytes())?;
    f.write_all(&4u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&(samples.len() as u32).to_le_bytes())?;
    f.write_all(&samples)?;
    println!("{out}: {seconds} s, left original, right corrected");
    Ok(())
}
