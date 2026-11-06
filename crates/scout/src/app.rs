//! The interactive program: the outer loop of the original (CODE 7) with
//! its phases, the keyboard and the window.

use std::time::{Duration, Instant};

use anyhow::Result;
use scout_core::sound::id as sound;
use scout_core::{Dir, Exit, Game, Key};
use scout_data::GameData;
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::{KeyboardState, Mod, Scancode};
use sdl3::pixels::PixelFormat;
use sdl3::render::{BlendMode, ScaleMode, Texture, TextureCreator};
use sdl3::sys::render::SDL_LOGICAL_PRESENTATION_INTEGER_SCALE;
use sdl3::video::{Window, WindowContext};

use crate::menu::{self, Action, Background, Item, Menu};
use crate::render::{self, Canvas};
use crate::settings::Settings;
use crate::text::{self, Line};
use crate::{DEFAULT_PALETTE, Shown, View, audio, draw, message_canvas};

/// Frames per second of the simulation. The original waits for the vertical
/// blank after every frame and runs at 60 frames per second on a Mac that is
/// fast enough (measured in Basilisk II, see re/tools/emulator/README.md).
const FRAMES_PER_SECOND: u32 = 60;

/// Keys for development (see the top of `main.rs`).
const DEVELOPER_KEYS: [Scancode; 5] = [
    Scancode::N,
    Scancode::P,
    Scancode::R,
    Scancode::Tab,
    Scancode::F1,
];

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

impl Phase {
    fn picture(id: i16) -> Self {
        Phase::Picture {
            id,
            key_down: false,
        }
    }

    fn screen(message: Shown) -> Self {
        Phase::Screen {
            message,
            key_down: false,
        }
    }

    fn level_screen() -> Self {
        Self::screen(Shown::Level)
    }

    fn title() -> Self {
        Phase::Menu(Menu::new(Background::Picture(menu::TITLE), false))
    }
}

/// Whether the program goes on after an event.
#[derive(PartialEq, Eq)]
enum Flow {
    Continue,
    Quit,
}

/// One picture for the window: RGB24 rows and the lines in the system font.
struct Picture {
    rgb: Vec<u8>,
    width: usize,
    height: usize,
    lines: Vec<Line>,
}

/// The state of the program, without the SDL objects that draw it.
struct App<'a> {
    data: &'a GameData,
    audio: audio::Audio,
    music: audio::Music,
    settings: Settings,
    dev: bool,
    phase: Phase,
    /// The level being played (from 0).
    level: usize,
    game: Game,
    /// NOWALL stays on until the program ends, as in the original.
    no_walls: bool,
    /// The game screen when it was paused.
    paused: Option<Canvas>,
    /// After the last level the end picture follows the PERFECT screen.
    end_picture: bool,
    /// What the developer keys show.
    view: View,
    palette: usize,
}

impl<'a> App<'a> {
    fn new(
        data: &'a GameData,
        audio: audio::Audio,
        music: audio::Music,
        settings: Settings,
        dev: bool,
    ) -> Result<Self> {
        Ok(Self {
            data,
            audio,
            music,
            settings,
            dev,
            phase: Phase::picture(menu::LOGO),
            level: 0,
            game: Game::new(data.levels[0].clone()),
            no_walls: false,
            paused: None,
            end_picture: false,
            view: View::Screen,
            palette: crate::palette_index(data, DEFAULT_PALETTE)?,
        })
    }

    fn playing(&self) -> bool {
        matches!(self.phase, Phase::Playing)
    }

    /// A new game from `level` (from 0).
    fn new_game(&mut self, level: usize) {
        self.level = level;
        self.game = Game::new(self.data.levels[level].clone());
        self.game.no_walls = self.no_walls;
    }

    /// Command (or Ctrl) + P, and the window losing the focus: the menu
    /// over the paused game.
    fn pause(&mut self) {
        if self.playing() {
            self.paused = Some(draw(self.data, &self.game, View::Screen));
            self.phase = Phase::Menu(Menu::new(Background::Paused, true));
        }
    }

    fn toggle_fullscreen(&mut self, window: &mut Window) {
        self.settings.fullscreen = !self.settings.fullscreen;
        self.settings.save();
        let _ = window.set_fullscreen(self.settings.fullscreen);
    }

    fn play_sound(&self, id: u8) {
        if let Some(sound) = self.data.sound(i16::from(id)) {
            self.audio.play(sound);
        }
    }

    /// A message after a level with its sound (CODE 7 ≈ 0x940 …).
    fn message(&mut self, message: Shown, sound: u8) {
        if self.settings.sounds {
            self.play_sound(sound);
        }
        self.phase = Phase::screen(message);
    }

    fn event(&mut self, event: Event, window: &mut Window) -> Flow {
        let action = match event {
            Event::Quit { .. } => Action::Quit,
            Event::Window {
                win_event: WindowEvent::FocusLost,
                ..
            } => {
                self.music.suspend();
                // unlike the original, the game pauses in the background
                self.pause();
                Action::None
            }
            Event::Window {
                win_event: WindowEvent::FocusGained,
                ..
            } => {
                self.music.resume();
                Action::None
            }
            Event::KeyDown {
                scancode: Some(code),
                keymod,
                repeat,
                ..
            } => self.key_down(code, keymod, repeat, window),
            Event::KeyUp { .. } | Event::MouseButtonUp { .. } => {
                self.released();
                Action::None
            }
            Event::MouseButtonDown { .. } => {
                match &mut self.phase {
                    Phase::Picture { key_down, .. } | Phase::Screen { key_down, .. } => {
                        *key_down = true;
                    }
                    Phase::Menu(m) if m.page == menu::Page::Closed => m.open(menu::Page::Main),
                    _ => {}
                }
                Action::None
            }
            _ => Action::None,
        };
        self.act(action, window)
    }

    fn key_down(
        &mut self,
        code: Scancode,
        keymod: Mod,
        repeat: bool,
        window: &mut Window,
    ) -> Action {
        if code == Scancode::F11 {
            self.toggle_fullscreen(window);
            return Action::None;
        }
        if let Some(c) = shortcut(keymod, code) {
            return self.shortcut(c, window);
        }
        let mut action = Action::None;
        let dev_key = self.dev && DEVELOPER_KEYS.contains(&code);
        match &mut self.phase {
            Phase::Menu(m) => action = m.key(code, &mut self.settings, self.data),
            Phase::Picture { key_down, .. } | Phase::Screen { key_down, .. }
                if !repeat && !dev_key =>
            {
                *key_down = true;
            }
            _ => {}
        }
        if self.dev && !repeat && !matches!(self.phase, Phase::Menu(_)) {
            self.developer_key(code);
        }
        action
    }

    /// The menu shortcuts of the original with Command (or Ctrl).
    fn shortcut(&mut self, c: char, window: &mut Window) -> Action {
        let playing = self.playing();
        match c {
            'Q' => Action::Quit,
            'F' => {
                self.toggle_fullscreen(window);
                Action::None
            }
            'P' if playing => {
                self.pause();
                Action::None
            }
            _ if playing => Action::None,
            'G' => Action::Play,
            'R' if self.paused.is_some() => Action::Resume,
            'I' | 'E' => {
                if let Phase::Menu(m) = &mut self.phase {
                    m.choose(if c == 'I' {
                        Item::Settings
                    } else {
                        Item::Password
                    });
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    /// N / P next / previous level, R restart the level, Tab game screen /
    /// level map, F1 palette.
    fn developer_key(&mut self, code: Scancode) {
        let n = self.data.levels.len();
        let new_level = match code {
            Scancode::N => Some((self.level + 1) % n),
            Scancode::P => Some((self.level + n - 1) % n),
            Scancode::R => Some(self.level),
            Scancode::Tab => {
                self.view = match self.view {
                    View::Screen => View::Map,
                    View::Map | View::Level => View::Screen,
                };
                None
            }
            Scancode::F1 => {
                self.palette = (self.palette + 1) % self.data.palettes.len();
                None
            }
            _ => None,
        };
        let on_level = matches!(
            self.phase,
            Phase::Playing
                | Phase::Screen {
                    message: Shown::Level,
                    ..
                }
        );
        if let Some(level) = new_level
            && on_level
        {
            self.new_game(level);
            self.phase = Phase::level_screen();
        }
    }

    /// A key or the mouse button released: pictures and screens go on.
    fn released(&mut self) {
        match self.phase {
            Phase::Picture { id, key_down: true } => {
                self.phase = if id == menu::END_PICTURE {
                    Phase::level_screen()
                } else {
                    Phase::title()
                };
            }
            Phase::Screen {
                message,
                key_down: true,
            } => {
                self.phase = match message {
                    Shown::Level => {
                        self.game.start_intro();
                        Phase::Playing
                    }
                    // the original goes back to its title with the game
                    // over picture (CODE 7 0xa86)
                    Shown::GameOver => Phase::Menu(Menu::new(
                        Background::Picture(menu::GAME_OVER_PICTURE),
                        false,
                    )),
                    _ => Phase::Quiet(message),
                };
            }
            _ => {}
        }
    }

    /// Do what the menu asks for.
    fn act(&mut self, action: Action, window: &mut Window) -> Flow {
        match action {
            Action::None => {}
            Action::Quit => return Flow::Quit,
            Action::Play | Action::Level(_) => {
                let level = match action {
                    Action::Level(level) => level,
                    _ => 0,
                };
                self.new_game(level);
                self.paused = None;
                self.end_picture = false;
                self.phase = Phase::level_screen();
            }
            Action::Resume => {
                if self.paused.take().is_some() {
                    self.phase = Phase::Playing;
                }
            }
            Action::NoWalls => {
                self.no_walls = true;
                self.game.no_walls = true;
            }
            Action::SettingsChanged => {
                let _ = window.set_fullscreen(self.settings.fullscreen);
                if self.settings.music && !self.music.is_playing() {
                    self.music.start(self.data);
                } else if !self.settings.music {
                    self.music.stop();
                }
            }
        }
        Flow::Continue
    }

    /// One frame of real time (1/60 s).
    fn frame(&mut self, keys: &KeyboardState) {
        match self.phase {
            Phase::Picture { .. } | Phase::Menu(_) | Phase::Screen { .. } => {}
            Phase::Quiet(_) => {
                if !self.audio.busy() {
                    self.phase = if std::mem::take(&mut self.end_picture) {
                        Phase::picture(menu::END_PICTURE)
                    } else {
                        Phase::level_screen()
                    };
                }
            }
            Phase::Playing => self.play_frame(keys),
        }
    }

    fn play_frame(&mut self, keys: &KeyboardState) {
        self.game.sounds.on = self.audio.is_on() && self.settings.sounds;
        self.game.sounds.busy = self.audio.busy();
        self.game.tick(key(keys, &self.settings));
        for id in std::mem::take(&mut self.game.sounds.start) {
            self.play_sound(id);
        }
        match self.game.exit {
            Some(Exit::Won) => {
                // after the last level the picture 1002, then the first
                // level again (CODE 7 0x9a4)
                self.level = (self.level + 1) % self.data.levels.len();
                self.end_picture = self.level == 0;
                self.game.next_level(self.data.levels[self.level].clone());
                self.message(Shown::Perfect, sound::PERFECT);
            }
            Some(Exit::Died) => {
                self.game.restart();
                self.message(Shown::TryAgain, sound::TRY_AGAIN);
            }
            Some(Exit::SelfDestructed) => {
                self.game.restart();
                self.message(Shown::Aborted, sound::ABORTED);
            }
            Some(Exit::GameOver) => {
                self.new_game(0);
                self.message(Shown::GameOver, sound::GAME_OVER);
            }
            None => {}
        }
    }

    /// What the window shows now.
    fn picture(&self) -> Picture {
        let data = self.data;
        let palette = &data.palettes[self.palette].palette;
        let screen = |screen: render::Screen, lines| Picture {
            rgb: screen.rgb,
            width: render::SCREEN_WIDTH,
            height: render::SCREEN_HEIGHT,
            lines,
        };
        let canvas = |image: Canvas| {
            let mut rgb = vec![0; image.width * image.height * 3];
            image.to_rgb24(palette, &mut rgb, image.width * 3);
            Picture {
                rgb,
                width: image.width,
                height: image.height,
                lines: vec![],
            }
        };
        match &self.phase {
            Phase::Picture { id, .. } => {
                let mut picture = render::Screen::black();
                picture.picture(data, *id, render::PICTURE_AREA);
                screen(picture, vec![])
            }
            Phase::Menu(m) => {
                let (picture, lines) = m.draw(data, &self.settings, self.paused.as_ref());
                screen(picture, lines)
            }
            Phase::Screen { message, .. } | Phase::Quiet(message) => {
                canvas(message_canvas(data, &self.game, self.level, *message))
            }
            Phase::Playing => canvas(draw(data, &self.game, self.view)),
        }
    }

    /// The window title; with `--dev` the level and the palette.
    fn title(&self) -> String {
        let shows_level = matches!(
            self.phase,
            Phase::Playing | Phase::Screen { .. } | Phase::Quiet(_)
        );
        if !(self.dev && shows_level) {
            return "S.C.Out".to_string();
        }
        let palette = &self.data.palettes[self.palette].name;
        let palette = if palette == DEFAULT_PALETTE {
            String::new()
        } else {
            format!(" – palette {palette}")
        };
        format!(
            "S.C.Out – level {}/{}{palette}",
            self.level + 1,
            self.data.levels.len()
        )
    }
}

/// The key of this frame. Like the original, only one key counts. Self
/// destruct is Esc in the original (CODE 9, virtual key code 0x35); here
/// it can be changed like the others.
fn key(keys: &KeyboardState, settings: &Settings) -> Key {
    let [right, left, up, down, get, fire, destruct] = settings.keys;
    [
        (left, Key::Move(Dir::Left)),
        (right, Key::Move(Dir::Right)),
        (down, Key::Move(Dir::Down)),
        (up, Key::Move(Dir::Up)),
        (get, Key::Use),
        (fire, Key::Fire),
        (destruct, Key::SelfDestruct),
    ]
    .into_iter()
    .find(|&(code, _)| keys.is_scancode_pressed(code))
    .map_or(Key::None, |(_, key)| key)
}

/// The letter of a menu shortcut: Command (or Ctrl) and a key with a
/// one-character name.
fn shortcut(keymod: Mod, code: Scancode) -> Option<char> {
    let command = keymod.intersects(Mod::LGUIMOD | Mod::RGUIMOD | Mod::LCTRLMOD | Mod::RCTRLMOD);
    command
        .then(|| menu::key_char(code))
        .flatten()
        .map(|c| c.to_ascii_uppercase())
}

/// A streaming texture filled with `pixels` (rows of `width` ×
/// `bytes_per_pixel` bytes).
fn upload_texture<'r>(
    creator: &'r TextureCreator<WindowContext>,
    format: PixelFormat,
    (width, height): (u32, u32),
    pixels: &[u8],
) -> Result<Texture<'r>> {
    let mut texture = creator.create_texture_streaming(format, width, height)?;
    let row = pixels.len() / height.max(1) as usize;
    texture.with_lock(None, |buf: &mut [u8], pitch: usize| {
        for (y, line) in pixels.chunks_exact(row).enumerate() {
            buf[y * pitch..y * pitch + row].copy_from_slice(line);
        }
    })?;
    Ok(texture)
}

/// Play: open the window and run until the player quits.
pub fn run(data: &GameData, dev: bool) -> Result<()> {
    let sdl = sdl3::init()?;
    let audio = audio::Audio::open(&sdl);
    let settings = Settings::load();
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
    if settings.music {
        music.start(data);
    }
    let mut canvas = window.into_canvas();
    if settings.fullscreen {
        let _ = canvas.window_mut().set_fullscreen(true);
    }
    let creator = canvas.texture_creator();
    let font = text::SystemFont::load();
    let mut app = App::new(data, audio, music, settings, dev)?;

    let frame_time = Duration::from_secs(1) / FRAMES_PER_SECOND;
    let mut next_frame = Instant::now();
    let mut title = String::new();
    // the lines in the system font, the size they were drawn at, and their
    // texture
    let mut text: Option<(Vec<Line>, (u32, u32), Texture)> = None;
    loop {
        for event in events.poll_iter() {
            if app.event(event, canvas.window_mut()) == Flow::Quit {
                return Ok(());
            }
        }

        app.music.fill();
        let now = Instant::now();
        while next_frame <= now {
            next_frame += frame_time;
            app.frame(&events.keyboard_state());
        }

        let picture = app.picture();
        let (w, h) = (picture.width as u32, picture.height as u32);
        let mut texture = upload_texture(&creator, PixelFormat::RGB24, (w, h), &picture.rgb)?;
        texture.set_scale_mode(ScaleMode::Nearest);
        canvas.set_logical_size(w, h, SDL_LOGICAL_PRESENTATION_INTEGER_SCALE)?;

        // the system font at the resolution of the window
        if picture.lines.is_empty() {
            text = None;
        } else {
            let (ow, oh) = canvas.output_size()?;
            // the picture is scaled by whole numbers (SDL_LOGICAL_PRESENTATION_INTEGER_SCALE)
            let scale = (ow / w).min(oh / h).max(1);
            let size = (w * scale, h * scale);
            if !text
                .as_ref()
                .is_some_and(|(lines, at, _)| *lines == picture.lines && *at == size)
            {
                let rgba = font.draw(&picture.lines, size.0 as usize, size.1 as usize);
                let mut t = upload_texture(&creator, PixelFormat::ABGR8888, size, &rgba)?;
                t.set_blend_mode(BlendMode::Blend);
                text = Some((picture.lines, size, t));
            }
        }

        let new_title = app.title();
        if new_title != title {
            canvas.window_mut().set_title(&new_title)?;
            title = new_title;
        }
        canvas.clear();
        canvas.copy(&texture, None, None)?;
        if let Some((_, _, t)) = &text {
            canvas.copy(t, None, None)?;
        }
        canvas.present();
        std::thread::sleep(next_frame.saturating_duration_since(Instant::now()));
    }
}
