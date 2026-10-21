//! S.C.Out for macOS, Windows and Linux (work in progress).
//!
//! Usage:
//!   scout [--dev] [DISK]                          play
//!   scout [DISK] --export LEVEL OUT.bmp [screen|map|level] [PALETTE]
//!   scout [DISK] --export-menu PAGE OUT.bmp       a menu page (see --help)
//!
//! DISK is the original `SCOUT_1_4.dsk`, the application or a data pack
//! (`scout-data.zip`, made by `scout-extract`). Without it, `SCOUT_DISK` is
//! used, then the one chosen before (saved in the settings), then
//! `scout-data.zip` beside the program or in the current folder, then
//! `SCOUT_1_4.dsk` in the current folder, then `re/work/SCOUT_1_4.dsk`; if
//! there is none, the game asks for it.
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

mod audio;
mod menu;
mod render;
mod settings;
mod text;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use menu::{Background, Menu};
use scout_core::sound::id as sound;
use scout_core::{Dir, Exit, Game, Key};
use scout_data::GameData;
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::{KeyboardState, Mod, Scancode};
use sdl3::pixels::PixelFormat;
use sdl3::render::ScaleMode;
use sdl3::sys::render::SDL_LOGICAL_PRESENTATION_INTEGER_SCALE;
use settings::Settings;

/// Palette shown first: the colors of the game screen.
const DEFAULT_PALETTE: &str = "game";
/// Frames per second of the simulation. The original waits for the vertical
/// blank after every frame and runs at 60 frames per second on a Mac that is
/// fast enough (measured in Basilisk II, see re/tools/emulator/README.md).
const FRAMES_PER_SECOND: u32 = 60;

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(at) = args.iter().position(|a| a == "--export-menu") {
        let rest: Vec<String> = args.drain(at..).skip(1).collect();
        if rest.len() < 2 {
            bail!(
                "usage: scout [DISK] --export-menu PAGE OUT.bmp (PAGE: {})",
                MENU_PAGES.join(", ")
            );
        }
        let disk = find_disk(args.first().map(PathBuf::from))?;
        let data = GameData::load(&disk).with_context(|| format!("loading {}", disk.display()))?;
        return export_menu(&data, &rest[0], Path::new(&rest[1]));
    }
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
    let dev = match args.iter().position(|a| a == "--dev") {
        Some(at) => {
            args.remove(at);
            true
        }
        None => false,
    };
    let Some(rest) = export else {
        return run(args.first().map(PathBuf::from), dev);
    };
    let disk = find_disk(args.first().map(PathBuf::from))?;
    let data = GameData::load(&disk).with_context(|| format!("loading {}", disk.display()))?;

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

/// The data pack `scout-extract` makes; the game looks for it beside the
/// program and in the current folder.
const PACK: &str = "scout-data.zip";

/// A file in the folder of the program.
fn beside_program(name: &str) -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join(name))
}

/// What the player sees when the game data is missing.
const NEED_DATA: &str = "S.C.Out needs the data of the original game (Kalisto, 1993): \
the disk image SCOUT_1_4.dsk, the S.C.OUT application itself (version 1.4 or 1.3.1) \
or a data pack made from them with scout-extract (scout-data.zip). They are not included.\n\nPlease choose the file in the next window.";

/// The game data: from the command line, `SCOUT_DISK`, the file chosen
/// before, the usual places, or else chosen by the player (`None` if they
/// cancel).
fn load_data(
    window: &sdl3::video::Window,
    events: &mut sdl3::EventPump,
    arg: Option<PathBuf>,
    settings: &mut Settings,
) -> Result<Option<GameData>> {
    let candidates = arg
        .into_iter()
        .chain(std::env::var_os("SCOUT_DISK").map(PathBuf::from))
        .chain(settings.disk.clone())
        .chain(beside_program(PACK))
        .chain([
            PathBuf::from(PACK),
            PathBuf::from("SCOUT_1_4.dsk"),
            PathBuf::from("re/work/SCOUT_1_4.dsk"),
        ]);
    for path in candidates.filter(|p| p.exists()) {
        match GameData::load(&path) {
            Ok(data) => return Ok(Some(data)),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }
    let mut message = NEED_DATA.to_string();
    loop {
        sdl3::messagebox::show_simple_message_box(
            sdl3::messagebox::MessageBoxFlag::INFORMATION,
            "S.C.Out",
            &message,
            window,
        )?;
        let Some(path) = choose_file(window, events) else {
            return Ok(None);
        };
        match GameData::load(&path) {
            Ok(data) => {
                settings.disk = Some(path);
                settings.save();
                return Ok(Some(data));
            }
            Err(e) => {
                message = format!(
                    "{} is not the S.C.Out game data ({e}).\n\n{NEED_DATA}",
                    path.display()
                );
            }
        }
    }
}

/// The system's dialog to open a file; `None` if the player cancels.
fn choose_file(window: &sdl3::video::Window, events: &mut sdl3::EventPump) -> Option<PathBuf> {
    use std::cell::RefCell;
    use std::rc::Rc;
    let chosen: Rc<RefCell<Option<Option<PathBuf>>>> = Rc::default();
    let result = chosen.clone();
    sdl3::dialog::show_open_file_dialog(
        &[],
        None::<&Path>,
        false,
        window,
        Box::new(move |files, _| {
            *result.borrow_mut() = Some(files.ok().and_then(|f| f.into_iter().next()));
        }),
    )
    .ok()?;
    // the answer comes while events are handled
    loop {
        if let Some(path) = chosen.borrow_mut().take() {
            return path;
        }
        events.pump_events();
        std::thread::sleep(Duration::from_millis(10));
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

/// Keys for development (see the top of this file).
const DEVELOPER_KEYS: [Scancode; 5] = [
    Scancode::N,
    Scancode::P,
    Scancode::R,
    Scancode::Tab,
    Scancode::F1,
];

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

/// Where the program is (the outer loop of the original, CODE 7).
enum Phase {
    /// A picture until a key or the mouse button is pressed and released:
    /// the logo before the title, the picture after the last level.
    Picture {
        id: i16,
        key_down: bool,
    },
    /// The title or the paused game with the menu (state 0 of CODE 7).
    Menu(Menu),
    /// A screen between levels until a key or the mouse button is pressed
    /// and released (CODE 7 0x10be).
    Screen {
        message: Shown,
        key_down: bool,
    },
    /// After a message: until its sound has ended, then the level screen.
    Quiet(Shown),
    Playing,
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
        key_down: false,
    }
}

fn title() -> Phase {
    Phase::Menu(Menu::new(Background::Picture(menu::TITLE), false))
}

/// A message after a level with its sound (CODE 7 ≈ 0x940 …).
fn message(
    audio: &audio::Audio,
    data: &GameData,
    settings: &Settings,
    message: Shown,
    sound: u8,
) -> Phase {
    if settings.sounds
        && let Some(s) = data.sounds.iter().find(|s| s.id == sound as i16)
    {
        audio.play(s);
    }
    Phase::Screen {
        message,
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

/// The key of this frame. Like the original, only one key counts. Self
/// destruct is Esc in the original (CODE 9, virtual key code 0x35); here
/// it can be changed like the others.
fn key(keys: &KeyboardState, settings: &Settings) -> Key {
    let [right, left, up, down, get, fire, destruct] = settings.keys;
    for (code, key) in [
        (left, Key::Move(Dir::Left)),
        (right, Key::Move(Dir::Right)),
        (down, Key::Move(Dir::Down)),
        (up, Key::Move(Dir::Up)),
        (get, Key::Use),
        (fire, Key::Fire),
        (destruct, Key::SelfDestruct),
    ] {
        if keys.is_scancode_pressed(code) {
            return key;
        }
    }
    Key::None
}

/// The menu shortcuts of the original with Command (or Ctrl).
fn shortcut(keymod: Mod, code: Scancode) -> Option<char> {
    let command = keymod.intersects(Mod::LGUIMOD | Mod::RGUIMOD | Mod::LCTRLMOD | Mod::RCTRLMOD);
    let name = code.name();
    let mut chars = name.chars();
    match (command, chars.next(), chars.next()) {
        (true, Some(c), None) => Some(c.to_ascii_uppercase()),
        _ => None,
    }
}

fn run(disk: Option<PathBuf>, dev: bool) -> Result<()> {
    let sdl = sdl3::init()?;
    let audio = audio::Audio::open(&sdl);
    let mut settings = Settings::load();
    let mut music = audio::Music::open(&sdl);
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
    let mut events = sdl.event_pump()?;
    let Some(data) = load_data(&window, &mut events, disk, &mut settings)? else {
        return Ok(());
    };
    let data = &data;
    music.fixed = settings.fixed_music;
    if settings.music {
        music.start(data, 1);
    }
    let mut canvas = window.into_canvas();
    if settings.fullscreen {
        let _ = canvas.window_mut().set_fullscreen(true);
    }
    let creator = canvas.texture_creator();
    let font = text::SystemFont::load();

    let mut level = 0usize;
    let mut game = Game::new(data.levels[level].clone());
    // NOWALL stays on until the program ends, as in the original
    let mut no_walls = false;
    // the game screen when it was paused
    let mut paused: Option<render::Canvas> = None;
    // after the last level the end picture follows the PERFECT screen
    let mut end_picture = false;
    let mut phase = Phase::Picture {
        id: menu::LOGO,
        key_down: false,
    };
    let mut view = View::Screen;
    let mut palette = palette_index(data, DEFAULT_PALETTE)?;
    let frame_time = Duration::from_secs(1) / FRAMES_PER_SECOND;
    let mut next_frame = Instant::now();
    let mut title_text = String::new();
    let mut shown_lines: Option<(Vec<text::Line>, (u32, u32))> = None;
    let mut text_texture = None;
    loop {
        for event in events.poll_iter() {
            let mut action = menu::Action::None;
            match event {
                Event::Quit { .. } => return Ok(()),
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => {
                    music.suspend();
                    // unlike the original, the game pauses in the background
                    if matches!(phase, Phase::Playing) {
                        paused = Some(draw(data, &game, View::Screen));
                        phase = Phase::Menu(Menu::new(Background::Paused, true));
                    }
                }
                Event::Window {
                    win_event: WindowEvent::FocusGained,
                    ..
                } => music.resume(),
                Event::KeyDown {
                    scancode: Some(code),
                    keymod,
                    repeat,
                    ..
                } => {
                    // Command (or Ctrl) + a letter: the menu shortcuts
                    if code == Scancode::F11 {
                        settings.fullscreen = !settings.fullscreen;
                        settings.save();
                        let _ = canvas.window_mut().set_fullscreen(settings.fullscreen);
                    } else if let Some(c) = shortcut(keymod, code) {
                        let playing = matches!(phase, Phase::Playing);
                        action = match c {
                            'Q' => menu::Action::Quit,
                            'F' => {
                                settings.fullscreen = !settings.fullscreen;
                                settings.save();
                                let _ = canvas.window_mut().set_fullscreen(settings.fullscreen);
                                menu::Action::None
                            }
                            'P' if playing => {
                                paused = Some(draw(data, &game, View::Screen));
                                phase = Phase::Menu(Menu::new(Background::Paused, true));
                                menu::Action::None
                            }
                            _ if playing => menu::Action::None,
                            'G' => menu::Action::Play,
                            'R' if paused.is_some() => menu::Action::Resume,
                            'I' | 'E' => {
                                if let Phase::Menu(m) = &mut phase {
                                    m.choose(if c == 'I' {
                                        menu::Item::Settings
                                    } else {
                                        menu::Item::Password
                                    });
                                }
                                menu::Action::None
                            }
                            _ => menu::Action::None,
                        };
                    } else {
                        match &mut phase {
                            Phase::Menu(m) => action = m.key(code, &mut settings, data),
                            Phase::Picture { key_down, .. } | Phase::Screen { key_down, .. }
                                if !repeat && !(dev && DEVELOPER_KEYS.contains(&code)) =>
                            {
                                *key_down = true
                            }
                            _ => {}
                        }
                        if dev && !repeat && !matches!(phase, Phase::Menu(_)) {
                            // developer keys
                            let n = data.levels.len();
                            let mut new_level = None;
                            match code {
                                Scancode::N => new_level = Some((level + 1) % n),
                                Scancode::P => new_level = Some((level + n - 1) % n),
                                Scancode::R => new_level = Some(level),
                                Scancode::Tab => {
                                    view = match view {
                                        View::Screen => View::Map,
                                        View::Map | View::Level => View::Screen,
                                    }
                                }
                                Scancode::F1 => palette = (palette + 1) % data.palettes.len(),
                                _ => {}
                            }
                            if let Some(l) = new_level
                                && matches!(
                                    phase,
                                    Phase::Playing
                                        | Phase::Screen {
                                            message: Shown::Level,
                                            ..
                                        }
                                )
                            {
                                level = l;
                                game = Game::new(data.levels[level].clone());
                                game.no_walls = no_walls;
                                phase = level_screen();
                            }
                        }
                    }
                }
                Event::KeyUp { .. } | Event::MouseButtonUp { .. } => match phase {
                    Phase::Picture { id, key_down: true } => {
                        phase = if id == menu::END_PICTURE {
                            level_screen()
                        } else {
                            title()
                        };
                    }
                    Phase::Screen {
                        message,
                        key_down: true,
                    } => {
                        // a key was pressed and released
                        phase = match message {
                            Shown::Level => {
                                game.start_intro();
                                Phase::Playing
                            }
                            // the original goes back to its title with the
                            // game over picture (CODE 7 0xa86)
                            Shown::GameOver => Phase::Menu(Menu::new(
                                Background::Picture(menu::GAME_OVER_PICTURE),
                                false,
                            )),
                            _ => Phase::Quiet(message),
                        };
                    }
                    _ => {}
                },
                Event::MouseButtonDown { .. } => match &mut phase {
                    Phase::Picture { key_down, .. } | Phase::Screen { key_down, .. } => {
                        *key_down = true
                    }
                    Phase::Menu(m) if m.page == menu::Page::Closed => m.open(menu::Page::Main),
                    _ => {}
                },
                _ => {}
            }
            match action {
                menu::Action::None => {}
                menu::Action::Quit => return Ok(()),
                menu::Action::Play | menu::Action::Level(_) => {
                    level = match action {
                        menu::Action::Level(l) => l,
                        _ => 0,
                    };
                    game = Game::new(data.levels[level].clone());
                    game.no_walls = no_walls;
                    paused = None;
                    end_picture = false;
                    phase = level_screen();
                }
                menu::Action::Resume => {
                    if paused.take().is_some() {
                        phase = Phase::Playing;
                    }
                }
                menu::Action::NoWalls => {
                    no_walls = true;
                    game.no_walls = true;
                }
                menu::Action::SettingsChanged => {
                    let _ = canvas.window_mut().set_fullscreen(settings.fullscreen);
                    music.set_fixed(settings.fixed_music);
                    if settings.music && !music.is_playing() {
                        // switched on: the original starts from position 0
                        music.start(data, 0);
                    } else if !settings.music {
                        music.stop();
                    }
                }
            }
        }

        music.fill();
        let now = Instant::now();
        while next_frame <= now {
            next_frame += frame_time;
            match &mut phase {
                Phase::Picture { .. } | Phase::Menu(_) | Phase::Screen { .. } => {}
                Phase::Quiet(_) => {
                    if !audio.busy() {
                        phase = if std::mem::take(&mut end_picture) {
                            Phase::Picture {
                                id: menu::END_PICTURE,
                                key_down: false,
                            }
                        } else {
                            level_screen()
                        };
                    }
                }
                Phase::Playing => {
                    game.sounds.on = audio.is_on() && settings.sounds;
                    game.sounds.busy = audio.busy();
                    game.tick(key(&events.keyboard_state(), &settings));
                    for id in game.sounds.start.drain(..) {
                        if let Some(s) = data.sounds.iter().find(|s| s.id == id as i16) {
                            audio.play(s);
                        }
                    }
                    match game.exit {
                        Some(Exit::Won) => {
                            // after the last level the picture 1002, then the
                            // first level again (CODE 7 0x9a4)
                            level = (level + 1) % data.levels.len();
                            end_picture = level == 0;
                            game.next_level(data.levels[level].clone());
                            phase =
                                message(&audio, data, &settings, Shown::Perfect, sound::PERFECT);
                        }
                        Some(exit @ (Exit::Died | Exit::SelfDestructed)) => {
                            game.restart();
                            phase = if exit == Exit::Died {
                                message(&audio, data, &settings, Shown::TryAgain, sound::TRY_AGAIN)
                            } else {
                                message(&audio, data, &settings, Shown::Aborted, sound::ABORTED)
                            };
                        }
                        Some(Exit::GameOver) => {
                            level = 0;
                            game = Game::new(data.levels[level].clone());
                            game.no_walls = no_walls;
                            phase =
                                message(&audio, data, &settings, Shown::GameOver, sound::GAME_OVER);
                        }
                        None => {}
                    }
                }
            }
        }

        let p = &data.palettes[palette];
        let (rgb, w, h, lines) = match &phase {
            Phase::Picture { id, .. } => {
                let mut screen = render::Screen::black();
                screen.picture(data, *id, render::PICTURE_AREA);
                (
                    screen.rgb,
                    render::SCREEN_WIDTH,
                    render::SCREEN_HEIGHT,
                    vec![],
                )
            }
            Phase::Menu(m) => {
                let (screen, lines) = m.draw(data, &settings, paused.as_ref());
                (
                    screen.rgb,
                    render::SCREEN_WIDTH,
                    render::SCREEN_HEIGHT,
                    lines,
                )
            }
            Phase::Screen { message, .. } | Phase::Quiet(message) => {
                let image = message_canvas(data, &game, level, *message);
                (
                    render::Screen::from_canvas(&image, &p.palette).rgb,
                    image.width,
                    image.height,
                    vec![],
                )
            }
            Phase::Playing => {
                let image = draw(data, &game, view);
                let mut rgb = vec![0; image.width * image.height * 3];
                image.to_rgb24(&p.palette, &mut rgb, image.width * 3);
                (rgb, image.width, image.height, vec![])
            }
        };
        let mut texture =
            creator.create_texture_streaming(PixelFormat::RGB24, w as u32, h as u32)?;
        texture.set_scale_mode(ScaleMode::Nearest);
        texture.with_lock(None, |buf: &mut [u8], pitch: usize| {
            for (row, line) in rgb.chunks_exact(w * 3).enumerate() {
                buf[row * pitch..row * pitch + w * 3].copy_from_slice(line);
            }
        })?;
        canvas.set_logical_size(w as u32, h as u32, SDL_LOGICAL_PRESENTATION_INTEGER_SCALE)?;

        // the system font at the resolution of the window
        if lines.is_empty() {
            text_texture = None;
            shown_lines = None;
        } else {
            let (ow, oh) = canvas.output_size()?;
            // the picture is scaled by whole numbers (SDL_LOGICAL_PRESENTATION_INTEGER_SCALE)
            let scale = (ow as f32 / w as f32)
                .min(oh as f32 / h as f32)
                .floor()
                .max(1.0);
            let size = ((w as f32 * scale) as u32, (h as f32 * scale) as u32);
            if shown_lines.as_ref() != Some(&(lines.clone(), size)) {
                let rgba = font.draw(&lines, size.0 as usize, size.1 as usize);
                let mut t =
                    creator.create_texture_streaming(PixelFormat::ABGR8888, size.0, size.1)?;
                t.set_blend_mode(sdl3::render::BlendMode::Blend);
                t.with_lock(None, |buf: &mut [u8], pitch: usize| {
                    let row = size.0 as usize * 4;
                    for (y, line) in rgba.chunks_exact(row).enumerate() {
                        buf[y * pitch..y * pitch + row].copy_from_slice(line);
                    }
                })?;
                text_texture = Some(t);
                shown_lines = Some((lines, size));
            }
        }

        let new_title = match phase {
            Phase::Playing | Phase::Screen { .. } | Phase::Quiet(_) if dev => format!(
                "S.C.Out – level {}/{}{}",
                level + 1,
                data.levels.len(),
                if p.name == DEFAULT_PALETTE {
                    String::new()
                } else {
                    format!(" – palette {}", p.name)
                }
            ),
            _ => "S.C.Out".to_string(),
        };
        if new_title != title_text {
            canvas.window_mut().set_title(&new_title)?;
            title_text = new_title;
        }
        canvas.clear();
        canvas.copy(&texture, None, None)?;
        if let Some(t) = &text_texture {
            canvas.copy(t, None, None)?;
        }
        canvas.present();
        std::thread::sleep(next_frame.saturating_duration_since(Instant::now()));
    }
}
