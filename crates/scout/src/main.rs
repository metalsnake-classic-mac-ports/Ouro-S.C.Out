//! Ouro S.C.Out: S.C.Out for macOS, Windows and Linux (work in progress).
//!
//! Usage:
//!   ouro-scout [--dev] [DATA]                     play
//!   ouro-scout [DATA] --export LEVEL OUT.bmp [screen|map|level] [PALETTE]
//!   ouro-scout [DATA] --export-menu PAGE OUT.bmp  a menu page
//!
//! The game data is built in (`data/ouro-scout-data.zip`). DATA, a data
//! pack made by `ouro-scout-extract`, replaces it; so does `SCOUT_DATA` or a
//! file `ouro-scout-data.zip` beside the program or in the current folder.
//!
//! Keys (changeable in the settings): arrows move, Return takes / drops /
//! teleports, Space fires, Esc blows yourself up. Command (or Ctrl) and P
//! pauses, Q quits; on the title G plays, R resumes, I opens the settings
//! and E the password, as in the original. F11 or Command (or Ctrl) and F
//! switches to full screen. The game pauses when its window loses the
//! focus.
//!
//! With `--dev`, keys for development: N/P next/previous level, R restart
//! the level, Tab game screen / level map, F1 palette.

mod app;
mod audio;
mod menu;
mod render;
mod settings;
mod text;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use menu::{Background, Menu};
use scout_core::Game;
use scout_data::GameData;
use settings::Settings;

/// Palette shown first: the colors of the game screen.
const DEFAULT_PALETTE: &str = "game";

const HELP: &str = "\
Ouro S.C.Out: S.C.Out (Kalisto, 1993) for modern systems.

usage: ouro-scout [--dev] [DATA]
       ouro-scout [DATA] --export LEVEL OUT.bmp [screen|map|level] [PALETTE]
       ouro-scout [DATA] --export-menu PAGE OUT.bmp
       ouro-scout --version | --help

The game data is built in; DATA, a data pack made by ouro-scout-extract,
replaces it (so do SCOUT_DATA and ouro-scout-data.zip beside the program
or in the current folder).

Keys: arrows move, Return takes / drops, Space fires, Esc blows yourself
up; Command (or Ctrl) + P pauses, + Q quits; F11 full screen. The keys can
be changed in the settings.";

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("ouro-scout {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let dev = take_flag(&mut args, "--dev");
    let menu_export = take_option(&mut args, "--export-menu");
    let level_export = take_option(&mut args, "--export");
    // what is left: the data pack
    let data = || load_data(args.first().map(PathBuf::from));
    if let Some(rest) = menu_export {
        let [page, out, ..] = rest.as_slice() else {
            bail!(
                "usage: ouro-scout [DATA] --export-menu PAGE OUT.bmp (PAGE: {})",
                MENU_PAGES.join(", ")
            );
        };
        return export_menu(&data()?, page, Path::new(out));
    }
    if let Some(rest) = level_export {
        let [level, out, more @ ..] = rest.as_slice() else {
            bail!("usage: ouro-scout [DATA] --export LEVEL OUT.bmp [screen|map|level] [PALETTE]");
        };
        let level: usize = level.parse().context("LEVEL must be a number from 1")?;
        let view = match more.first().map(String::as_str) {
            None | Some("screen") => View::Screen,
            Some("map") => View::Map,
            Some("level") => View::Level,
            Some(other) => bail!("unknown view {other:?}, use screen, map or level"),
        };
        let palette = more.get(1).map_or(DEFAULT_PALETTE, String::as_str);
        return export_level(&data()?, level, view, palette, Path::new(out));
    }
    app::run(&data()?, dev)
}

/// Remove `flag` from the arguments; whether it was there.
fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let at = args.iter().position(|a| a == flag);
    at.map(|at| args.remove(at)).is_some()
}

/// Remove `option` and the arguments after it; those arguments.
fn take_option(args: &mut Vec<String>, option: &str) -> Option<Vec<String>> {
    let at = args.iter().position(|a| a == option)?;
    Some(args.drain(at..).skip(1).collect())
}

/// The data pack `ouro-scout-extract` makes (`data/ouro-scout-data.zip`),
/// built into the program.
const BUNDLED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/ouro-scout-data.zip"
));
/// A pack of this name beside the program or in the current folder is used
/// instead of the built-in one.
const PACK: &str = "ouro-scout-data.zip";

/// A file in the folder of the program.
fn beside_program(name: &str) -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join(name))
}

/// The game data: a pack from the command line, `SCOUT_DATA`, beside the
/// program or in the current folder, else the built-in one.
fn load_data(arg: Option<PathBuf>) -> Result<GameData> {
    let candidates = arg
        .into_iter()
        .chain(std::env::var_os("SCOUT_DATA").map(PathBuf::from))
        .chain(beside_program(PACK))
        .chain([PathBuf::from(PACK)]);
    for path in candidates.filter(|p| p.exists()) {
        match GameData::load(&path) {
            Ok(data) => return Ok(data),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }
    GameData::from_pack(BUNDLED).context("the built-in game data")
}

fn palette_index(data: &GameData, name: &str) -> Result<usize> {
    data.palette_index(name).with_context(|| {
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

const MENU_PAGES: [&str; 9] = [
    "logo", "title", "menu", "pause", "settings", "keys", "password", "wrong", "about",
];

/// A menu page as the window shows it at twice the size, with the system
/// font (for looking at them without playing).
fn export_menu(data: &GameData, page: &str, out: &Path) -> Result<()> {
    let settings = Settings::default();
    let mut game = Game::new(data.levels[0].clone());
    game.skip_intro();
    let paused = draw(data, &game, View::Screen);
    let (screen, lines) = if page == "logo" {
        let mut screen = render::Screen::black();
        screen.picture(data, menu::LOGO, render::PICTURE_AREA);
        (screen, vec![])
    } else {
        let mut m = Menu::new(Background::Picture(menu::TITLE), false);
        match page {
            "title" => {}
            "menu" => m.open(menu::Page::Main),
            "pause" => {
                m = Menu::new(Background::Paused, true);
                m.open(menu::Page::Main);
            }
            "settings" => {
                m.open(menu::Page::Settings);
                m.selected = 3;
            }
            "keys" => {
                m.open(menu::Page::Keys { waiting: false });
                m.selected = 4;
            }
            "password" => m.open(menu::Page::Password {
                text: "ABC".into(),
                wrong: false,
            }),
            "wrong" => m.open(menu::Page::Password {
                text: "ABCDEF".into(),
                wrong: true,
            }),
            "about" => m.page = menu::Page::About,
            _ => bail!("unknown page {page:?}; pages: {}", MENU_PAGES.join(", ")),
        }
        m.draw(data, &settings, Some(&paused))
    };
    let (w, h) = (2 * render::SCREEN_WIDTH, 2 * render::SCREEN_HEIGHT);
    let text = text::SystemFont::load().draw(&lines, w, h);
    let mut rgb = vec![0; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let src = ((y / 2) * render::SCREEN_WIDTH + x / 2) * 3;
            let t = &text[(y * w + x) * 4..(y * w + x) * 4 + 4];
            let a = t[3] as u32;
            for c in 0..3 {
                rgb[(y * w + x) * 3 + c] =
                    ((screen.rgb[src + c] as u32 * (255 - a) + t[c] as u32 * a) / 255) as u8;
            }
        }
    }
    render::write_bmp(out, w, h, &rgb).with_context(|| format!("writing {}", out.display()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shown {
    Level,
    TryAgain,
    Aborted,
    GameOver,
    Perfect,
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
