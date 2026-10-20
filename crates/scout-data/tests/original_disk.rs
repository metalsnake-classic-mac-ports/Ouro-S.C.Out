//! Tests against the original disk image. The image is not part of the
//! repository; set `SCOUT_DISK` or put it at `re/work/SCOUT_1_4.dsk`.
//! Without it the tests are skipped.

use std::path::PathBuf;

use scout_core::Level;
use scout_data::GameData;

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
    Some(GameData::load(&path).expect("original disk image loads"))
}

#[test]
fn counts_match_the_original() {
    let Some(game) = load() else { return };
    assert_eq!(game.levels.len(), 101);
    assert_eq!(game.tiles.len(), 480);
    assert_eq!(game.objects.len(), 39);
    assert_eq!(game.font.len(), 45);
    assert_eq!(game.icons.len(), 24);
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
    assert_eq!(game.level_codes[67], "MUFUPO"); // level 69, the port says MUFUMO
    assert_eq!(game.level_codes[99], "ZETIPO"); // level 101
}

#[test]
fn levels_match_the_port_where_it_was_not_edited() {
    let Some(game) = load() else { return };
    let port = std::fs::read(repo_root().join("legacy-port/LEVEL.DAT")).unwrap();
    // Levels the port changed with its editor (0-based).
    let edited = [7, 9, 67, 70, 79, 85, 93, 94, 98];
    for (n, level) in game.levels.iter().enumerate() {
        let chunk = &port[2 + n * 2880..2 + (n + 1) * 2880];
        let same = (0..Level::HEIGHT).all(|y| {
            (0..Level::WIDTH).all(|x| {
                let i = y * Level::WIDTH + x;
                level.file_tile(x, y) == u16::from_le_bytes([chunk[2 * i], chunk[2 * i + 1]])
                    && level.file_floor(x, y) == chunk[2 * Level::CELLS + i]
            })
        });
        assert_eq!(same, !edited.contains(&n), "level {}", n + 1);
    }
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
