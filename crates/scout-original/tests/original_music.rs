//! The music player against the original one running in an emulator
//! (`re/tools/emu_music.py <work> 420 re/work/music/start<N>.raw "" <N>`
//! for start positions 0 and 1). Without the recordings the test is skipped.

use std::path::PathBuf;

use scout_original::MusicVersion;
use scout_original::music::{MusicPlayer, TICK_SAMPLES};

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
    let data =
        scout_original::load(&disk, MusicVersion::Original).expect("original disk image loads");
    for start in [0u8, 1] {
        let reference = std::fs::read(dir.join(format!("start{start}.raw"))).unwrap();
        let mut player = MusicPlayer::new(&data.module, &data.volumes, start);
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

/// The corrected player differs (the module uses E93), but only a little.
#[test]
fn fixed_music_differs_from_the_original() {
    let disk = repo_root().join("re/work/SCOUT_1_4.dsk");
    if !disk.exists() {
        eprintln!("skipped: disk image not found");
        return;
    }
    let data =
        scout_original::load(&disk, MusicVersion::Original).expect("original disk image loads");
    let mut original = MusicPlayer::new(&data.module, &data.volumes, 0);
    let mut fixed = MusicPlayer::new(&data.module, &data.volumes, 0);
    fixed.fixed = true;
    let (mut differ, mut total) = (0, 0);
    for _ in 0..10_000 {
        let (a, b) = (original.tick(), fixed.tick());
        differ += a.iter().zip(&b).filter(|(a, b)| a != b).count();
        total += TICK_SAMPLES;
    }
    assert!(differ > 0);
    assert!(differ < total, "{differ} of {total}");
}

/// The tables the player builds are the ones in the original code: sine,
/// periods, steps (periods 0–856) and the volume table.
#[test]
fn tables_match_the_original_code() {
    let code = repo_root().join("re/work/code/seg10.bin");
    let Ok(code) = std::fs::read(&code) else {
        eprintln!("skipped: {} not found", code.display());
        return;
    };
    let volumes = scout_original::music::volumes_from_code(&code).expect("volume table");
    let built = scout_original::music::tables(&volumes);
    for (name, range) in [
        ("sine", 0x3E..0x5E),
        ("periods", 0x5E..0xD0),
        ("steps", 0x9D8..0x173C),
        ("volumes", 0x1740..0x5740),
    ] {
        assert_eq!(built[range.clone()], code[range.clone()], "{name}");
    }
}
