//! The music player against the original one running in an emulator
//! (`re/tools/emu_music.py <work> 420 re/work/music/start<N>.raw "" <N>`
//! for start positions 0 and 1). Without the recordings the test is skipped.

use std::path::PathBuf;

use scout_core::music::{MusicPlayer, TICK_SAMPLES};
use scout_data::GameData;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn music_matches_the_original() {
    let disk = repo_root().join("re/work/SCOUT_1_4.dsk");
    let dir = repo_root().join("re/work/music");
    if !disk.exists() || !dir.exists() {
        eprintln!("skipped: disk image or recordings not found");
        return;
    }
    let data = GameData::load(&disk).expect("original disk image loads");
    for start in [0u8, 1] {
        let reference = std::fs::read(dir.join(format!("start{start}.raw"))).unwrap();
        let mut player = MusicPlayer::new(&data.music, &data.music_code, start);
        for (tick, expected) in reference.as_chunks::<TICK_SAMPLES>().0.iter().enumerate() {
            let got = player.tick();
            if let Some(i) = (0..TICK_SAMPLES).find(|&i| got[i] != expected[i]) {
                panic!(
                    "start {start}, tick {tick}, sample {i}: {} instead of {}",
                    got[i], expected[i]
                );
            }
        }
    }
}
