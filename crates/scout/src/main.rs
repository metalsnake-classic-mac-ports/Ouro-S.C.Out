//! S.C.Out level viewer (first milestone of the new implementation).
//!
//! Usage:
//!   scout [DISK]                          show the levels in a window
//!   scout [DISK] --export LEVEL OUT.bmp [PALETTE]
//!
//! DISK is the original `SCOUT_1_4.dsk`. Without it, `SCOUT_DISK` is used,
//! then `SCOUT_1_4.dsk` in the current folder, then `re/work/SCOUT_1_4.dsk`.
//!
//! Keys: ←/→ level, ↑/↓ palette, Esc quit.

mod render;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use scout_data::GameData;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::render::ScaleMode;
use sdl3::sys::render::SDL_LOGICAL_PRESENTATION_LETTERBOX;

/// Palette shown first. The real in-game palette is not known yet (see
/// TODO.md); this one at least has the green floor of the box screenshots.
const DEFAULT_PALETTE: &str = "PICT 1000";

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let export = match args.iter().position(|a| a == "--export") {
        Some(at) => {
            let rest: Vec<String> = args.drain(at..).skip(1).collect();
            if rest.len() < 2 {
                bail!("usage: scout [DISK] --export LEVEL OUT.bmp [PALETTE]");
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
            let palette = rest.get(2).map(String::as_str).unwrap_or(DEFAULT_PALETTE);
            export_level(&game, level, palette, &PathBuf::from(&rest[1]))
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

fn export_level(game: &GameData, level: usize, palette: &str, out: &PathBuf) -> Result<()> {
    let data = game
        .levels
        .get(level.wrapping_sub(1))
        .with_context(|| format!("level {level} does not exist (1..={})", game.levels.len()))?;
    let palette = &game.palettes[palette_index(game, palette)?].palette;
    let mut rgb = vec![0; render::WIDTH * render::HEIGHT * 3];
    render::to_rgb24(
        &render::level_indices(game, data),
        palette,
        &mut rgb,
        render::WIDTH * 3,
    );
    render::write_bmp(out, &rgb).with_context(|| format!("writing {}", out.display()))
}

fn run_viewer(game: &GameData) -> Result<()> {
    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let window = video
        .window("S.C.Out", render::WIDTH as u32, render::HEIGHT as u32)
        .resizable()
        .position_centered()
        .build()?;
    let mut canvas = window.into_canvas();
    canvas.set_logical_size(
        render::WIDTH as u32,
        render::HEIGHT as u32,
        SDL_LOGICAL_PRESENTATION_LETTERBOX,
    )?;
    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_streaming(
        PixelFormat::RGB24,
        render::WIDTH as u32,
        render::HEIGHT as u32,
    )?;
    texture.set_scale_mode(ScaleMode::Nearest);

    let mut level = 0usize;
    let mut palette = palette_index(game, DEFAULT_PALETTE)?;
    let mut indices = render::level_indices(game, &game.levels[level]);
    let mut events = sdl.event_pump()?;
    let mut redraw = true;
    loop {
        if redraw {
            let p = &game.palettes[palette];
            texture.with_lock(None, |buf: &mut [u8], pitch: usize| {
                render::to_rgb24(&indices, &p.palette, buf, pitch)
            })?;
            canvas.window_mut().set_title(&format!(
                "S.C.Out – level {}/{} – palette {} (←/→ level, ↑/↓ palette)",
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
                    _ => continue,
                }
                indices = render::level_indices(game, &game.levels[level]);
                redraw = true;
            }
            Event::Window { .. } => redraw = true,
            _ => {}
        }
    }
}
