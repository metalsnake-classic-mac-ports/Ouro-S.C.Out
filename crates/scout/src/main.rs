//! S.C.Out for macOS, Windows and Linux (work in progress).
//!
//! Usage:
//!   scout [DISK]                                  play
//!   scout [DISK] --export LEVEL OUT.bmp [screen|map|level] [PALETTE]
//!
//! DISK is the original `SCOUT_1_4.dsk`. Without it, `SCOUT_DISK` is used,
//! then `SCOUT_1_4.dsk` in the current folder, then `re/work/SCOUT_1_4.dsk`.
//!
//! Keys: arrows move, Return takes / drops / teleports, Space fires,
//! Backspace blows yourself up, N/P next/previous level, R restart the
//! level, Tab game screen / level map, F1 palette, Esc quit.

mod render;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use scout_core::{Dir, Exit, Game, Key};
use scout_data::GameData;
use sdl3::event::Event;
use sdl3::keyboard::{KeyboardState, Keycode, Scancode};
use sdl3::pixels::PixelFormat;
use sdl3::render::ScaleMode;
use sdl3::sys::render::SDL_LOGICAL_PRESENTATION_LETTERBOX;

/// Palette shown first: the colors of the game screen.
const DEFAULT_PALETTE: &str = "game";
/// Frames per second of the simulation. The original waits for the vertical
/// blank after every frame and runs at 60 frames per second on a Mac that is
/// fast enough (measured in Basilisk II, see re/tools/emulator/README.md).
const FRAMES_PER_SECOND: u32 = 60;

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let export = match args.iter().position(|a| a == "--export") {
        Some(at) => {
            let rest: Vec<String> = args.drain(at..).skip(1).collect();
            if rest.len() < 2 {
                bail!("usage: scout [DISK] --export LEVEL OUT.bmp [screen|map|level] [PALETTE]");
            }
            Some(rest)
        }
        None => None,
    };
    let disk = find_disk(args.first().map(PathBuf::from))?;
    let data = GameData::load(&disk).with_context(|| format!("loading {}", disk.display()))?;

    match export {
        Some(rest) => {
            let level: usize = rest[0].parse().context("LEVEL must be a number from 1")?;
            let view = match rest.get(2).map(String::as_str) {
                None | Some("screen") => View::Screen,
                Some("map") => View::Map,
                Some("level") => View::Level,
                Some(other) => bail!("unknown view {other:?}, use screen, map or level"),
            };
            let palette = rest.get(3).map(String::as_str).unwrap_or(DEFAULT_PALETTE);
            export_level(&data, level, view, palette, &PathBuf::from(&rest[1]))
        }
        None => run(&data),
    }
}

fn find_disk(arg: Option<PathBuf>) -> Result<PathBuf> {
    let candidates = arg
        .into_iter()
        .chain(std::env::var_os("SCOUT_DISK").map(PathBuf::from))
        .chain([
            PathBuf::from("SCOUT_1_4.dsk"),
            PathBuf::from("re/work/SCOUT_1_4.dsk"),
        ]);
    for path in candidates {
        if path.exists() {
            return Ok(path);
        }
    }
    bail!("original disk image SCOUT_1_4.dsk not found; pass its path as the first argument")
}

fn palette_index(data: &GameData, name: &str) -> Result<usize> {
    data.palettes
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case(name))
        .with_context(|| {
            let names: Vec<_> = data.palettes.iter().map(|p| p.name.as_str()).collect();
            format!("unknown palette {name:?}; available: {}", names.join(", "))
        })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    /// The 640 × 480 game screen.
    Screen,
    /// The whole level.
    Map,
    /// The level screen before playing (export only).
    Level,
}

fn draw(data: &GameData, game: &Game, view: View) -> render::Canvas {
    match view {
        View::Screen => render::game_screen(data, game),
        View::Map | View::Level => render::level_map(data, game),
    }
}

fn export_level(
    data: &GameData,
    level: usize,
    view: View,
    palette: &str,
    out: &Path,
) -> Result<()> {
    if level == 0 || level > data.levels.len() {
        bail!("level {level} does not exist (1..={})", data.levels.len());
    }
    let palette = &data.palettes[palette_index(data, palette)?].palette;
    let mut game = Game::new(data.levels[level - 1].clone());
    game.skip_intro();
    let canvas = match view {
        View::Level => message_canvas(data, &game, level - 1, Shown::Level),
        _ => draw(data, &game, view),
    };
    let mut rgb = vec![0; canvas.width * canvas.height * 3];
    canvas.to_rgb24(palette, &mut rgb, canvas.width * 3);
    render::write_bmp(out, canvas.width, canvas.height, &rgb)
        .with_context(|| format!("writing {}", out.display()))
}

/// Frames a message shows after the end of a level. The original waits
/// for the end of its sound there; until there is sound, two seconds.
const MESSAGE_FRAMES: u32 = 2 * FRAMES_PER_SECOND;

/// Where the game is between levels (the outer loop of the original, CODE 7).
enum Phase {
    /// A screen; `wait` frames left, or 0: until a key is pressed and
    /// released (CODE 7 0x10be).
    Screen {
        message: Shown,
        wait: u32,
        key_down: bool,
    },
    Playing,
}

#[derive(Clone, Copy)]
enum Shown {
    Level,
    TryAgain,
    Aborted,
    GameOver,
    Perfect,
}

fn level_screen() -> Phase {
    Phase::Screen {
        message: Shown::Level,
        wait: 0,
        key_down: false,
    }
}

fn message(message: Shown) -> Phase {
    Phase::Screen {
        message,
        wait: MESSAGE_FRAMES,
        key_down: false,
    }
}

fn message_canvas(data: &GameData, game: &Game, level: usize, shown: Shown) -> render::Canvas {
    let message = match shown {
        Shown::Level => render::Message::Level(
            level + 1,
            level.checked_sub(1).map(|i| data.level_codes[i].as_str()),
            game.player.lives,
        ),
        Shown::TryAgain => render::Message::TryAgain,
        Shown::Aborted => render::Message::Aborted,
        Shown::GameOver => render::Message::GameOver,
        Shown::Perfect => render::Message::Perfect,
    };
    render::message_screen(data, &message)
}

/// The key of this frame. Like the original, only one key counts.
fn key(keys: &KeyboardState) -> Key {
    for (code, key) in [
        (Scancode::Left, Key::Move(Dir::Left)),
        (Scancode::Right, Key::Move(Dir::Right)),
        (Scancode::Down, Key::Move(Dir::Down)),
        (Scancode::Up, Key::Move(Dir::Up)),
        (Scancode::Return, Key::Use),
        (Scancode::Space, Key::Fire),
        (Scancode::Backspace, Key::SelfDestruct),
    ] {
        if keys.is_scancode_pressed(code) {
            return key;
        }
    }
    Key::None
}

fn run(data: &GameData) -> Result<()> {
    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let window = video
        .window(
            "S.C.Out",
            2 * render::SCREEN_WIDTH as u32,
            2 * render::SCREEN_HEIGHT as u32,
        )
        .resizable()
        .position_centered()
        .build()?;
    let mut canvas = window.into_canvas();
    let creator = canvas.texture_creator();

    let mut level = 0usize;
    let mut game = Game::new(data.levels[level].clone());
    let mut phase = level_screen();
    let mut view = View::Screen;
    let mut palette = palette_index(data, DEFAULT_PALETTE)?;
    let mut events = sdl.event_pump()?;
    let frame_time = Duration::from_secs(1) / FRAMES_PER_SECOND;
    let mut next_frame = Instant::now();
    let mut title = String::new();
    loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Ok(()),
                Event::KeyDown {
                    keycode: Some(key),
                    repeat: false,
                    ..
                } => {
                    let n = data.levels.len();
                    match key {
                        Keycode::N => level = (level + 1) % n,
                        Keycode::P => level = (level + n - 1) % n,
                        Keycode::R => {}
                        _ if matches!(phase, Phase::Screen { wait: 0, .. }) => {
                            if let Phase::Screen { key_down, .. } = &mut phase {
                                *key_down = true;
                            }
                            continue;
                        }
                        Keycode::Tab => {
                            view = match view {
                                View::Screen => View::Map,
                                View::Map | View::Level => View::Screen,
                            };
                            continue;
                        }
                        Keycode::F1 => {
                            palette = (palette + 1) % data.palettes.len();
                            continue;
                        }
                        _ => continue,
                    }
                    game = Game::new(data.levels[level].clone());
                    phase = level_screen();
                }
                Event::KeyUp { .. } | Event::MouseButtonUp { .. } => {
                    if let Phase::Screen {
                        wait: 0,
                        key_down: true,
                        ..
                    } = phase
                    {
                        // a key was pressed and released: start playing
                        game.start_intro();
                        phase = Phase::Playing;
                    }
                }
                Event::MouseButtonDown { .. } => {
                    if let Phase::Screen {
                        wait: 0, key_down, ..
                    } = &mut phase
                    {
                        *key_down = true;
                    }
                }
                _ => {}
            }
        }

        let now = Instant::now();
        while next_frame <= now {
            next_frame += frame_time;
            match &mut phase {
                Phase::Screen { wait: 0, .. } => {}
                Phase::Screen { wait, .. } => {
                    *wait -= 1;
                    if *wait == 0 {
                        phase = level_screen();
                    }
                }
                Phase::Playing => {
                    game.tick(key(&events.keyboard_state()));
                    match game.exit {
                        Some(Exit::Won) => {
                            // after the last level the original goes on with the first
                            level = (level + 1) % data.levels.len();
                            game.next_level(data.levels[level].clone());
                            phase = message(Shown::Perfect);
                        }
                        Some(exit @ (Exit::Died | Exit::SelfDestructed)) => {
                            game.restart();
                            phase = message(if exit == Exit::Died {
                                Shown::TryAgain
                            } else {
                                Shown::Aborted
                            });
                        }
                        Some(Exit::GameOver) => {
                            // the original shows a picture and goes back to its
                            // title; we start again with the first level
                            level = 0;
                            game = Game::new(data.levels[level].clone());
                            phase = message(Shown::GameOver);
                        }
                        None => {}
                    }
                }
            }
        }

        let image = match phase {
            Phase::Screen { message, .. } => message_canvas(data, &game, level, message),
            Phase::Playing => draw(data, &game, view),
        };
        let p = &data.palettes[palette];
        let (w, h) = (image.width as u32, image.height as u32);
        let mut texture = creator.create_texture_streaming(PixelFormat::RGB24, w, h)?;
        texture.set_scale_mode(ScaleMode::Nearest);
        texture.with_lock(None, |buf: &mut [u8], pitch: usize| {
            image.to_rgb24(&p.palette, buf, pitch)
        })?;
        canvas.set_logical_size(w, h, SDL_LOGICAL_PRESENTATION_LETTERBOX)?;
        let new_title = format!(
            "S.C.Out – level {}/{}{} (arrows move, N/P level, R restart, Tab map)",
            level + 1,
            data.levels.len(),
            if p.name == DEFAULT_PALETTE {
                String::new()
            } else {
                format!(" – palette {}", p.name)
            }
        );
        if new_title != title {
            canvas.window_mut().set_title(&new_title)?;
            title = new_title;
        }
        canvas.clear();
        canvas.copy(&texture, None, None)?;
        canvas.present();
        std::thread::sleep(next_frame.saturating_duration_since(Instant::now()));
    }
}
