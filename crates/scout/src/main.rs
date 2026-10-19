//! S.C.Out level viewer (first milestone of the new implementation).
//!
//! Usage:
//!   scout [DISK]                          show the levels in a window
//!   scout [DISK] --export LEVEL OUT.bmp [screen|map] [PALETTE]
//!
//! DISK is the original `SCOUT_1_4.dsk`. Without it, `SCOUT_DISK` is used,
//! then `SCOUT_1_4.dsk` in the current folder, then `re/work/SCOUT_1_4.dsk`.
//!
//! Keys: ←/→ level, Tab game screen / level map, ↑/↓ palette, Esc quit.

mod render;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use scout_data::GameData;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::render::ScaleMode;
use sdl3::sys::render::SDL_LOGICAL_PRESENTATION_LETTERBOX;

/// Palette shown first: the colors of the game screen.
const DEFAULT_PALETTE: &str = "game";

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let export = match args.iter().position(|a| a == "--export") {
        Some(at) => {
            let rest: Vec<String> = args.drain(at..).skip(1).collect();
            if rest.len() < 2 {
                bail!("usage: scout [DISK] --export LEVEL OUT.bmp [screen|map] [PALETTE]");
            }
            Some(rest)
        }
        None => None,
    };
    let disk = find_disk(args.first().map(PathBuf::from))?;
    let game = GameData::load(&disk).with_context(|| format!("loading {}", disk.display()))?;

    match export {
        Some(rest) => {
            let level: usize = rest[0].parse().context("LEVEL must be a number from 1")?;
            let view = match rest.get(2).map(String::as_str) {
                None | Some("screen") => View::Screen,
                Some("map") => View::Map,
                Some(other) => bail!("unknown view {other:?}, use screen or map"),
            };
            let palette = rest.get(3).map(String::as_str).unwrap_or(DEFAULT_PALETTE);
            export_level(&game, level, view, palette, &PathBuf::from(&rest[1]))
        }
        None => run_viewer(&game),
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

fn palette_index(game: &GameData, name: &str) -> Result<usize> {
    game.palettes
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case(name))
        .with_context(|| {
            let names: Vec<_> = game.palettes.iter().map(|p| p.name.as_str()).collect();
            format!("unknown palette {name:?}; available: {}", names.join(", "))
        })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    /// The 640 × 480 game screen at the start of the level.
    Screen,
    /// The whole level.
    Map,
}

fn draw(game: &GameData, level: usize, view: View) -> render::Canvas {
    let level = &game.levels[level];
    match view {
        View::Screen => render::start_screen(game, level),
        View::Map => render::level_map(game, level),
    }
}

fn export_level(
    game: &GameData,
    level: usize,
    view: View,
    palette: &str,
    out: &Path,
) -> Result<()> {
    if level == 0 || level > game.levels.len() {
        bail!("level {level} does not exist (1..={})", game.levels.len());
    }
    let palette = &game.palettes[palette_index(game, palette)?].palette;
    let canvas = draw(game, level - 1, view);
    let mut rgb = vec![0; canvas.width * canvas.height * 3];
    canvas.to_rgb24(palette, &mut rgb, canvas.width * 3);
    render::write_bmp(out, canvas.width, canvas.height, &rgb)
        .with_context(|| format!("writing {}", out.display()))
}

fn run_viewer(game: &GameData) -> Result<()> {
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
    let mut view = View::Screen;
    let mut palette = palette_index(game, DEFAULT_PALETTE)?;
    let mut image = draw(game, level, view);
    let mut events = sdl.event_pump()?;
    let mut redraw = true;
    loop {
        if redraw {
            let p = &game.palettes[palette];
            let (w, h) = (image.width as u32, image.height as u32);
            let mut texture = creator.create_texture_streaming(PixelFormat::RGB24, w, h)?;
            texture.set_scale_mode(ScaleMode::Nearest);
            texture.with_lock(None, |buf: &mut [u8], pitch: usize| {
                image.to_rgb24(&p.palette, buf, pitch)
            })?;
            canvas.set_logical_size(w, h, SDL_LOGICAL_PRESENTATION_LETTERBOX)?;
            canvas.window_mut().set_title(&format!(
                "S.C.Out – level {}/{} – palette {} (←/→ level, Tab screen/map, ↑/↓ palette)",
                level + 1,
                game.levels.len(),
                p.name
            ))?;
            canvas.clear();
            canvas.copy(&texture, None, None)?;
            canvas.present();
            redraw = false;
        }
        match events.wait_event() {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => return Ok(()),
            Event::KeyDown {
                keycode: Some(key), ..
            } => {
                let n = game.levels.len();
                let p = game.palettes.len();
                match key {
                    Keycode::Right => level = (level + 1) % n,
                    Keycode::Left => level = (level + n - 1) % n,
                    Keycode::Up => palette = (palette + 1) % p,
                    Keycode::Down => palette = (palette + p - 1) % p,
                    Keycode::Tab => {
                        view = match view {
                            View::Screen => View::Map,
                            View::Map => View::Screen,
                        }
                    }
                    _ => continue,
                }
                image = draw(game, level, view);
                redraw = true;
            }
            Event::Window { .. } => redraw = true,
            _ => {}
        }
    }
}
