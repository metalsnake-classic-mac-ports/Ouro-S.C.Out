//! A data pack made from the original disk gives the same game data.
//! Without the disk image the test is skipped.

use std::path::PathBuf;

use scout_core::music::{MusicPlayer, TICK_SAMPLES};
use scout_data::{GameData, Palette};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The colors of palette indices (a pack may pick another index of the
/// same color).
fn rgb(palette: &Palette, pixels: &[u8]) -> Vec<(u8, u8, u8)> {
    pixels
        .iter()
        .map(|&i| {
            let c = palette.colors[i as usize];
            (c.r, c.g, c.b)
        })
        .collect()
}

#[test]
fn a_pack_holds_the_same_data() {
    let disk = repo_root().join("re/work/SCOUT_1_4.dsk");
    if !disk.exists() {
        eprintln!("skipped: disk image not found");
        return;
    }
    let a = GameData::load(&disk).expect("original disk image loads");
    let pack = a.to_pack().expect("pack");
    let b = GameData::from_pack(&pack).expect("pack loads");
    let (pa, pb) = (&a.game_palette, &b.game_palette);
    assert_eq!(pa, pb);

    assert!(a.levels == b.levels, "levels");
    assert_eq!(a.level_codes, b.level_codes);
    assert_eq!(a.tiles.len(), b.tiles.len());
    for i in 0..a.tiles.len() {
        assert_eq!(
            rgb(pa, a.tiles.tile(i).unwrap()),
            rgb(pb, b.tiles.tile(i).unwrap()),
            "tile {i}"
        );
    }
    for i in 0..a.box_tiles.len() {
        assert_eq!(
            rgb(pa, a.box_tiles.tile(i).unwrap()),
            rgb(pb, b.box_tiles.tile(i).unwrap())
        );
    }
    for (name, sa, sb) in [
        ("objects", &a.objects, &b.objects),
        ("font", &a.font, &b.font),
        ("small font", &a.small_font, &b.small_font),
    ] {
        assert_eq!(sa.len(), sb.len(), "{name}");
        let n = sa.size();
        for i in 0..sa.len() {
            for p in 0..n * n {
                let (x, y) = (p % n, p / n);
                let color = |pal: &Palette, v: Option<u8>| v.map(|v| rgb(pal, &[v])[0]);
                assert_eq!(
                    color(pa, sa.pixel(i, x, y)),
                    color(pb, sb.pixel(i, x, y)),
                    "{name} {i}"
                );
            }
        }
    }
    for (x, y) in a.icons.iter().zip(&b.icons) {
        assert_eq!(rgb(pa, &x.pixels), rgb(pb, &y.pixels));
    }
    assert_eq!(a.icons.len(), b.icons.len());
    for (x, y) in [
        (&a.frame.top, &b.frame.top),
        (&a.frame.bottom, &b.frame.bottom),
        (&a.frame.left, &b.frame.left),
        (&a.frame.right, &b.frame.right),
    ] {
        assert_eq!(rgb(pa, &x.pixels), rgb(pb, &y.pixels));
    }
    assert_eq!(a.pictures.len(), b.pictures.len());
    for (x, y) in a.pictures.iter().zip(&b.pictures) {
        assert_eq!(x.id, y.id);
        assert_eq!(
            rgb(&x.image.palette, &x.image.pixels),
            rgb(&y.image.palette, &y.image.pixels),
            "picture {}",
            x.id
        );
    }
    assert_eq!(a.sounds.len(), b.sounds.len());
    for (x, y) in a.sounds.iter().zip(&b.sounds) {
        assert_eq!((x.id, x.rate, &x.samples), (y.id, y.rate, &y.samples));
    }
    assert_eq!(a.music, b.music);
    assert_eq!(a.unused_music, b.unused_music);
    assert_eq!(a.music_volumes, b.music_volumes);

    // and the music from the pack is the original's
    let reference = repo_root().join("re/work/music/start1.raw");
    if let Ok(reference) = std::fs::read(reference) {
        let mut player = MusicPlayer::new(&b.music, &b.music_volumes, 1);
        for expected in reference.as_chunks::<TICK_SAMPLES>().0.iter().take(3000) {
            assert_eq!(&player.tick(), expected);
        }
    }
}
