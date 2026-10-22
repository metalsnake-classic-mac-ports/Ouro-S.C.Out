//! Tests against the original disk image. The image is not part of the
//! repository; set `SCOUT_DISK` or put it at `re/work/SCOUT_1_4.dsk`.
//! Without it the tests are skipped.

use std::path::PathBuf;

use scout_data::GameData;
use scout_original::MusicVersion;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load() -> Option<GameData> {
    let path = std::env::var_os("SCOUT_DISK")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join("re/work/SCOUT_1_4.dsk"));
    if !path.exists() {
        eprintln!(
            "skipped: original disk image not found at {}",
            path.display()
        );
        return None;
    }
    Some(
        scout_original::load(&path, MusicVersion::Original)
            .expect("original disk image loads")
            .game,
    )
}

#[test]
fn counts_match_the_original() {
    let Some(game) = load() else { return };
    assert_eq!(game.levels.len(), 101);
    assert_eq!(game.tiles.len(), 480);
    assert_eq!(game.objects.len(), 39);
    assert_eq!(game.font.len(), 45);
    assert_eq!(game.small_font.len(), 45);
    assert_eq!(game.icons.len(), 24);
    assert_eq!(game.sounds.len(), 25);
    assert!(
        game.sounds
            .iter()
            .all(|s| s.rate == 11127 || s.rate == 5564)
    );
    assert_eq!(game.box_tiles.len(), 9);
    assert_eq!(game.level_codes.len(), 100);
    assert_eq!(game.pictures.len(), 5);
}

#[test]
fn level_1_starts_like_in_the_screenshot() {
    let Some(game) = load() else { return };
    assert_eq!(game.levels[0].start(), Some((2, 2)));
    assert_eq!(game.levels[0].start_view(), (0, 0));
}

#[test]
fn every_level_has_a_start() {
    let Some(game) = load() else { return };
    for (n, level) in game.levels.iter().enumerate() {
        assert!(level.start().is_some(), "level {}", n + 1);
    }
}

#[test]
fn level_codes() {
    let Some(game) = load() else { return };
    assert_eq!(game.level_codes[0], "RAFAFE"); // level 2
    assert_eq!(game.level_codes[67], "MUFUPO"); // level 69 (the Codebook says MUFUMO)
    assert_eq!(game.level_codes[99], "ZETIPO"); // level 101
}

#[test]
fn pictures_have_the_expected_sizes() {
    let Some(game) = load() else { return };
    let sizes: Vec<_> = game
        .pictures
        .iter()
        .map(|p| (p.id, p.image.width, p.image.height))
        .collect();
    assert_eq!(
        sizes,
        [
            (256, 320, 200),
            (1000, 512, 380),
            (1001, 519, 380),
            (1002, 519, 381),
            (2000, 512, 384)
        ]
    );
}
