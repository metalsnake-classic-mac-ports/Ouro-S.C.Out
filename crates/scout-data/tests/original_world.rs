//! Compares the world automaton with the original, frame by frame.
//!
//! The reference comes from running the original code in an emulator
//! (`re/tools/emu_world.py <work> all <frames> re/work/reference`). It is not
//! part of the repository; without it (or without the disk image) the test is
//! skipped.

use std::path::PathBuf;

use scout_core::world::{Kind, Projectile};
use scout_core::{Command, Game, Level};
use scout_data::GameData;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct BoxState {
    tile: u16,
    floor: u16,
    skip: bool,
}

/// Reference changes per frame: frame → [(box, state)].
fn read_reference(text: &str) -> Reference {
    let mut frames: Vec<Vec<(usize, BoxState)>> = Vec::new();
    let mut flags = Vec::new();
    let mut shots = Vec::new();
    let mut wall_hits = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("w ") {
            let (i, n) = rest.split_once(' ').unwrap();
            wall_hits.push((i.parse().unwrap(), n.parse().unwrap()));
            continue;
        }
        if let Some(rest) = line.strip_prefix("g ") {
            let v: Vec<i64> = rest.split(' ').map(|n| n.parse().unwrap()).collect();
            flags.push((v[0] as usize, v[1..].try_into().unwrap()));
            continue;
        }
        if let Some(rest) = line.strip_prefix("p ") {
            let v: Vec<i64> = rest.split(' ').map(|n| n.parse().unwrap()).collect();
            let projectile = Projectile {
                active: v[2] != 0,
                draw: v[3] as u8,
                life: v[4] as i16,
                x: v[5] as i32,
                y: v[6] as i32,
                vx: v[7] as i32,
                vy: v[8] as i32,
                // never used slots have kind 0
                kind: match v[9] {
                    2 => Kind::Ricochet,
                    3 => Kind::Homing,
                    _ => Kind::Normal,
                },
            };
            shots.push((v[0] as usize, v[1] as usize, projectile));
            continue;
        }
        let v: Vec<i64> = line.split(' ').map(|n| n.parse().unwrap()).collect();
        let frame = v[0] as usize;
        if frames.len() <= frame {
            frames.resize(frame + 1, Vec::new());
        }
        let state = BoxState {
            tile: v[2] as u16,
            floor: v[3] as u16,
            skip: v[4] != 0,
        };
        frames[frame].push((v[1] as usize, state));
    }
    // frames without box changes at the end still count
    let last = flags
        .iter()
        .map(|f| f.0)
        .chain(shots.iter().map(|s| s.0))
        .max();
    if let Some(last) = last.filter(|&l| l >= frames.len()) {
        frames.resize(last + 1, Vec::new());
    }
    Reference {
        frames,
        flags,
        shots,
        wall_hits,
    }
}

/// Global state of the world: no_death, dying, level_end, switches,
/// projectile count, forced command, player hit, random seed.
type Flags = [i64; 12];

/// Reference changes per frame: frame → [(box, state)], (frame, flags) and
/// (frame, slot, projectile).
struct Reference {
    frames: Vec<Vec<(usize, BoxState)>>,
    flags: Vec<(usize, Flags)>,
    shots: Vec<(usize, usize, Projectile)>,
    /// Hits reinforced walls took before (box, hits).
    wall_hits: Vec<(usize, u16)>,
}

/// The header line: `# player X Y switches S0..S4 carried C pulse P frame F
/// view X Y seed S`; puts the state into `game`.
fn apply_header(game: &mut Game, head: &str) {
    let h: Vec<&str> = head.split(' ').collect();
    let num = |at: usize| h[at].parse::<usize>().unwrap();
    for s in 0..5 {
        game.world.switches[s] = num(5 + s) != 0;
    }
    game.world.carried = num(11) as u16;
    game.world.pulse = num(13) as u16;
    game.world.frame = num(15);
    game.view = (num(17), num(18));
    game.world.random.seed = num(20) as u32;
}

fn flags_of(game: &Game) -> Flags {
    let w = &game.world;
    let s = |i: usize| w.switches[i] as i64;
    [
        w.no_death as i64,
        w.dying as i64,
        w.level_end as i64,
        s(0),
        s(1),
        s(2),
        s(3),
        s(4),
        w.projectile_count as i64,
        w.forced_command as i64,
        w.player_hit as i64,
        w.random.seed as i64,
    ]
}

fn state_of(game: &Game) -> Vec<BoxState> {
    (0..Level::CELLS)
        .map(|i| {
            let (x, y) = (i % Level::WIDTH, i / Level::WIDTH);
            BoxState {
                tile: game.level.tile(x, y),
                floor: game.level.floor(x, y),
                skip: game.world.skip(x, y),
            }
        })
        .collect()
}

/// Play the frames on `game` and compare with the reference; returns the
/// first difference.
fn compare(mut game: Game, reference: &Reference, label: &str) -> Option<String> {
    let mut expected_flags = flags_of(&game);
    let mut expected = vec![
        BoxState {
            tile: 0,
            floor: 0,
            skip: false
        };
        Level::CELLS
    ];
    let start = state_of(&game);
    let mut expected_shots = [Projectile::default(); 40];
    let mut shots = reference.shots.iter().peekable();
    for (frame, changes) in reference.frames.iter().enumerate() {
        for &(i, state) in changes {
            expected[i] = state;
        }
        while let Some(&(_, slot, p)) = shots.next_if(|(at, _, _)| *at == frame) {
            expected_shots[slot] = p;
        }
        if frame == 0 {
            game.world.projectiles = expected_shots;
            for &(i, hits) in &reference.wall_hits {
                game.world.wall_hits[i] = hits;
            }
        } else {
            game.tick(Command::None);
        }
        if let Some(slot) = (0..40).find(|&i| game.world.projectiles[i] != expected_shots[i]) {
            return Some(format!(
                "{label}: frame {frame}: projectile {slot}: expected {:?}, got {:?}",
                expected_shots[slot], game.world.projectiles[slot]
            ));
        }
        if let Some((_, f)) = reference.flags.iter().rev().find(|(at, _)| *at == frame) {
            expected_flags = *f;
        }
        if flags_of(&game) != expected_flags {
            return Some(format!(
                "{label}: frame {frame}: flags expected {expected_flags:?}, got {:?}",
                flags_of(&game)
            ));
        }
        let actual = state_of(&game);
        if let Some(i) = (0..Level::CELLS).find(|&i| actual[i] != expected[i]) {
            return Some(format!(
                "{label}: frame {frame}, box ({}, {}): expected {:?}, got {:?} (start tile {})",
                i % Level::WIDTH,
                i / Level::WIDTH,
                expected[i],
                actual[i],
                start[i].tile,
            ));
        }
    }
    None
}

#[test]
fn world_matches_the_original() {
    let disk = repo_root().join("re/work/SCOUT_1_4.dsk");
    let reference_dir = repo_root().join("re/work/reference");
    if !disk.exists() || !reference_dir.exists() {
        eprintln!("skipped: disk image or reference not found");
        return;
    }
    let data = GameData::load(&disk).unwrap();
    let mut failures = Vec::new();
    let mut compared = 0;
    for (n, level) in data.levels.iter().enumerate() {
        let path = reference_dir.join(format!("world_{:03}.txt", n + 1));
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        compared += 1;
        let frames = read_reference(&text);
        let label = format!("level {}", n + 1);
        let mut game = Game::new(level.clone());
        apply_header(&mut game, text.lines().next().unwrap());
        failures.extend(compare(game, &frames, &label));
    }
    assert!(
        compared > 0,
        "no reference files in {}",
        reference_dir.display()
    );
    assert!(
        failures.is_empty(),
        "{} of {compared} levels differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Random scenarios (`re/tools/emu_fuzz.py <work> <count> <frames> re/work/fuzz`).
#[test]
fn world_matches_the_original_in_random_scenarios() {
    let dir = repo_root().join("re/work/fuzz");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        eprintln!("skipped: no scenarios in {}", dir.display());
        return;
    };
    let mut paths: Vec<_> = entries.map(|e| e.unwrap().path()).collect();
    paths.sort();
    let mut failures = Vec::new();
    for path in &paths {
        let text = std::fs::read_to_string(path).unwrap();
        let (head, body) = text.split_once('\n').unwrap();
        let h: Vec<&str> = head.split(' ').collect();
        let num = |at: usize| h[at].parse::<usize>().unwrap();
        let frames = read_reference(body);
        let mut tiles = [0; Level::CELLS];
        let mut floor = [0; Level::CELLS];
        for &(i, state) in &frames.frames[0] {
            tiles[i] = state.tile;
            floor[i] = state.floor;
        }
        let level = Level::from_translated(tiles, floor, Some((num(2), num(3))));
        let mut game = Game::new(level);
        apply_header(&mut game, head);
        game.world.projectile_count = frames.flags[0].1[8] as i32;
        let label = path.file_name().unwrap().to_string_lossy().into_owned();
        failures.extend(compare(game, &frames, &label));
    }
    assert!(
        failures.is_empty(),
        "{} of {} scenarios differ:\n{}",
        failures.len(),
        paths.len(),
        failures.join("\n")
    );
}
