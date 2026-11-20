//! The interactive program: the outer loop of the original (CODE 7) with
//! its phases, the keyboard, the mouse and the window - and the Ouro
//! ports' rules on top of it (`../CLAUDE.md`, "How every port plays"):
//!
//! 1. Deaths, not lives: a death restarts from the last checkpoint as in
//!    the original, but lives never run out (no GAME OVER); the level
//!    screen shows the deaths of the game.
//! 2. No passwords: the level screen names the level instead of showing
//!    its code; the level selection replaces the password dialog.
//! 3. The level selection with the records (`records.json`): a run is
//!    from the level's start to its target destroyed, deaths included;
//!    its time counts the game's frames (60 a second) while playing.
//! 4. Continue: leaving the program keeps the game under way
//!    (`continue.json`, with the snapshots in `continue/`).
//! 5. Rewind: a snapshot every 5 seconds of play; Backspace (or Timeline
//!    in the paused menu) shows them as a tree, Return plays on from one.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use ouro_play::picture::Picture as Png;
use ouro_play::records::{Records, clock};
use ouro_play::rewind::Rewind;
use ouro_ui::Rect;
use scout_core::sound::id as sound;
use scout_core::{Dir, Exit, Game, Key};
use scout_data::GameData;
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::{KeyboardState, Mod, Scancode};
use sdl3::pixels::PixelFormat;
use sdl3::render::{BlendMode, ScaleMode, Texture, TextureCreator, WindowCanvas};
use sdl3::sys::render::{SDL_LOGICAL_PRESENTATION_INTEGER_SCALE, SDL_LOGICAL_PRESENTATION_LETTERBOX};
use sdl3::video::WindowContext;
use serde::{Deserialize, Serialize};

use crate::menu::{self, Action, Background, Item, Menu};
use crate::render::{self, Canvas};
use crate::settings::{self, Settings};
use crate::text::{self, Line, SystemFont};
use crate::theme::Look;
use crate::{DEFAULT_PALETTE, Shown, View, audio, draw, message_canvas};

/// Frames per second of the simulation. The original waits for the vertical
/// blank after every frame and runs at 60 frames per second on a Mac that is
/// fast enough (measured in Basilisk II, see re/tools/emulator/README.md).
const FRAMES_PER_SECOND: u32 = 60;
/// A snapshot every 5 seconds of play; at most an hour of them kept whole.
const SNAPSHOT_EVERY: u64 = 5 * FRAMES_PER_SECOND as u64;
const SNAPSHOTS: usize = 720;
/// The format of Continue's file and the snapshots (raised when [`Snap`]
/// or the core's state changes so an older file cannot be read).
const FORMAT: u32 = 1;
const SCREEN: Rect = Rect::new(0, 0, render::SCREEN_WIDTH as i32, render::SCREEN_HEIGHT as i32);

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
    /// The snapshots' tree (the game paused behind); `from_menu`: Esc goes
    /// back to the paused menu, else to the game.
    Timeline {
        view: ouro_ui::timeline::Timeline,
        from_menu: bool,
    },
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

/// What Continue and a snapshot keep: the core's game and what the port
/// counts beside it.
#[derive(Clone, Serialize, Deserialize)]
struct Snap {
    level: usize,
    game: Game,
    no_walls: bool,
    deaths: u32,
    run_frames: u32,
    run_deaths: u32,
    /// The game's frames played (when the next snapshot is due)
    frames: u64,
    /// In play (else on the screens between levels: Continue shows the
    /// level screen)
    playing: bool,
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
    /// NOWALL stays on until the game ends, as in the original.
    no_walls: bool,
    /// The game screen when it was paused.
    paused: Option<Canvas>,
    /// After the last level the end picture follows the PERFECT screen.
    end_picture: bool,
    /// What the developer keys show.
    view: View,
    palette: usize,
    /// Deaths in this game; the run of this level: its frames and deaths.
    deaths: u32,
    run_frames: u32,
    run_deaths: u32,
    /// A game is under way (Continue keeps it when the program is left).
    in_game: bool,
    records: Records,
    rewind: Rewind<Snap>,
    frames: u64,
    /// The level selection's start views.
    previews: RefCell<HashMap<usize, Png>>,
}

fn folder() -> Option<PathBuf> {
    settings::folder()
}

fn continue_file() -> Option<PathBuf> {
    folder().map(|d| d.join("continue.json"))
}

fn snapshots_folder() -> Option<PathBuf> {
    folder().map(|d| d.join("continue"))
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
            deaths: 0,
            run_frames: 0,
            run_deaths: 0,
            in_game: false,
            records: Records::load(folder().map(|d| d.join("records.json")), FRAMES_PER_SECOND),
            rewind: Rewind::new(SNAPSHOT_EVERY, SNAPSHOTS),
            frames: 0,
            previews: RefCell::new(HashMap::new()),
        })
    }

    fn playing(&self) -> bool {
        matches!(self.phase, Phase::Playing)
    }

    /// The title's menu (Continue when a game is kept).
    fn title_menu(background: i16) -> Menu {
        let mut m = Menu::new(Background::Picture(background), false);
        m.saved = continue_file().is_some_and(|p| p.exists());
        m
    }

    /// A new game from `level` (from 0).
    fn new_game(&mut self, level: usize) {
        self.level = level;
        self.game = Game::new(self.data.levels[level].clone());
        self.game.no_walls = self.no_walls;
        self.deaths = 0;
        self.run_frames = 0;
        self.run_deaths = 0;
        self.frames = 0;
        self.rewind.clear();
        self.in_game = true;
    }

    fn snap(&self) -> Snap {
        Snap {
            level: self.level,
            game: self.game.clone(),
            no_walls: self.no_walls,
            deaths: self.deaths,
            run_frames: self.run_frames,
            run_deaths: self.run_deaths,
            frames: self.frames,
            playing: self.playing() || self.paused.is_some(),
        }
    }

    fn apply(&mut self, s: Snap) {
        self.level = s.level.min(self.data.levels.len() - 1);
        self.game = s.game;
        self.no_walls = s.no_walls;
        self.deaths = s.deaths;
        self.run_frames = s.run_frames;
        self.run_deaths = s.run_deaths;
        self.frames = s.frames;
        self.paused = None;
        self.end_picture = false;
        self.in_game = true;
        self.phase = if s.playing { Phase::Playing } else { Phase::level_screen() };
    }

    /// Leaving the program: the game under way is kept for Continue.
    fn keep(&mut self) {
        if !self.in_game {
            return;
        }
        let (Some(file), Some(dir)) = (continue_file(), snapshots_folder()) else { return };
        if let Err(e) = ouro_play::saved::save(&file, FORMAT, &self.snap()) {
            eprintln!("{}: {e}", file.display());
        }
        if let Err(e) = self.rewind.save(&dir) {
            eprintln!("{}: {e}", dir.display());
        }
    }

    /// Continue: the game kept when the program was left.
    fn continue_game(&mut self) {
        let Some(file) = continue_file() else { return };
        let Some(snap) = ouro_play::saved::load::<Snap>(&file, ouro_play::saved::only(FORMAT)) else {
            if let Phase::Menu(m) = &mut self.phase {
                m.saved = false;
            }
            return;
        };
        self.rewind = snapshots_folder()
            .and_then(|d| Rewind::load(&d, SNAPSHOT_EVERY, SNAPSHOTS).map_err(|e| eprintln!("{}: {e}", d.display())).ok())
            .unwrap_or_else(|| Rewind::new(SNAPSHOT_EVERY, SNAPSHOTS));
        self.apply(snap);
    }

    /// Command (or Ctrl) + P, and the window losing the focus: the menu
    /// over the paused game.
    fn pause(&mut self) {
        if self.playing() {
            self.paused = Some(draw(self.data, &self.game, View::Screen));
            let mut m = Menu::new(Background::Paused, true);
            m.open(menu::Page::Main);
            self.phase = Phase::Menu(m);
        }
    }

    /// The timeline over the paused game.
    fn open_timeline(&mut self, from_menu: bool) {
        if self.paused.is_none() {
            self.paused = Some(draw(self.data, &self.game, View::Screen));
        }
        let mut view = ouro_ui::timeline::Timeline::default();
        view.open(&self.rewind.tree(), self.rewind.current());
        self.phase = Phase::Timeline { view, from_menu };
    }

    fn toggle_fullscreen(&mut self, canvas: &mut WindowCanvas) {
        self.settings.fullscreen = !self.settings.fullscreen;
        self.settings.save();
        let _ = canvas.window_mut().set_fullscreen(self.settings.fullscreen);
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

    fn event(&mut self, event: Event, canvas: &mut WindowCanvas) -> Flow {
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
            } => self.key_down(code, keymod, repeat, canvas),
            Event::KeyUp { .. } | Event::MouseButtonUp { .. } => {
                self.released();
                Action::None
            }
            Event::MouseMotion { x, y, .. } => self.mouse(canvas, x, y, false),
            Event::MouseButtonDown { x, y, .. } => {
                if let Phase::Picture { key_down, .. } | Phase::Screen { key_down, .. } = &mut self.phase {
                    *key_down = true;
                }
                self.mouse(canvas, x, y, true)
            }
            Event::MouseWheel { y, .. } => {
                let levels = self.data.levels.len();
                if let Phase::Menu(m) = &mut self.phase {
                    m.wheel(y, levels);
                }
                Action::None
            }
            _ => Action::None,
        };
        self.act(action, canvas)
    }

    /// The mouse at (x, y) of the window.
    fn mouse(&mut self, canvas: &WindowCanvas, x: f32, y: f32, click: bool) -> Action {
        let (x, y) = screen_point(canvas, x, y);
        let levels = self.data.levels.len();
        match &mut self.phase {
            Phase::Menu(m) => m.mouse(x, y, click, &mut self.settings, levels),
            Phase::Timeline { view, .. } => match view.mouse(x, y, click, &self.rewind.tree(), SCREEN) {
                Some(ouro_ui::timeline::Choice::Play(k)) => {
                    self.rewind_to(k);
                    Action::None
                }
                _ => Action::None,
            },
            _ => Action::None,
        }
    }

    fn key_down(
        &mut self,
        code: Scancode,
        keymod: Mod,
        repeat: bool,
        canvas: &mut WindowCanvas,
    ) -> Action {
        let waiting = matches!(&self.phase, Phase::Menu(m) if m.waiting());
        if code == Scancode::F11 && !waiting {
            self.toggle_fullscreen(canvas);
            return Action::None;
        }
        if !waiting && let Some(c) = shortcut(keymod, code) {
            return self.shortcut(c, canvas);
        }
        let mut action = Action::None;
        let dev_key = self.dev && DEVELOPER_KEYS.contains(&code);
        let levels = self.data.levels.len();
        match &mut self.phase {
            Phase::Menu(m) => action = m.key(code, &mut self.settings, levels),
            Phase::Timeline { view, from_menu } => {
                let from_menu = *from_menu;
                match menu::ui_key(code).and_then(|k| view.key(k, &self.rewind.tree())) {
                    Some(ouro_ui::timeline::Choice::Play(k)) => self.rewind_to(k),
                    Some(ouro_ui::timeline::Choice::Back) if from_menu => {
                        let mut m = Menu::new(Background::Paused, true);
                        m.open(menu::Page::Main);
                        self.phase = Phase::Menu(m);
                    }
                    Some(ouro_ui::timeline::Choice::Back) => {
                        self.paused = None;
                        self.phase = Phase::Playing;
                    }
                    None => {}
                }
            }
            Phase::Playing if code == Scancode::Backspace && !repeat => self.open_timeline(false),
            Phase::Picture { key_down, .. } | Phase::Screen { key_down, .. }
                if !repeat && !dev_key =>
            {
                *key_down = true;
            }
            _ => {}
        }
        if self.dev && !repeat && !matches!(self.phase, Phase::Menu(_) | Phase::Timeline { .. }) {
            self.developer_key(code);
        }
        action
    }

    /// Play on from snapshot `k` of the timeline.
    fn rewind_to(&mut self, k: usize) {
        if let Some(s) = self.rewind.restore(k) {
            self.apply(Snap { playing: true, ..s });
        }
    }

    /// The menu shortcuts of the original with Command (or Ctrl).
    fn shortcut(&mut self, c: char, canvas: &mut WindowCanvas) -> Action {
        let playing = self.playing();
        match c {
            'Q' => Action::Quit,
            'F' => {
                self.toggle_fullscreen(canvas);
                Action::None
            }
            'P' if playing => {
                self.pause();
                Action::None
            }
            _ if playing => Action::None,
            'G' => Action::NewGame,
            'R' if self.paused.is_some() => Action::Resume,
            'I' => {
                if let Phase::Menu(m) = &mut self.phase {
                    m.choose(Item::Options);
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
                    Phase::Menu(Self::title_menu(menu::TITLE))
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
                    _ => Phase::Quiet(message),
                };
            }
            _ => {}
        }
    }

    /// Do what the menu asks for.
    fn act(&mut self, action: Action, canvas: &mut WindowCanvas) -> Flow {
        match action {
            Action::None => {}
            Action::Quit => {
                self.keep();
                return Flow::Quit;
            }
            Action::NewGame | Action::Level(_) => {
                let level = match action {
                    Action::Level(level) => level,
                    _ => 0,
                };
                self.new_game(level);
                self.paused = None;
                self.end_picture = false;
                self.phase = Phase::level_screen();
            }
            Action::Continue => self.continue_game(),
            Action::Timeline => self.open_timeline(true),
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
                let _ = canvas.window_mut().set_fullscreen(self.settings.fullscreen);
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
            Phase::Picture { .. } | Phase::Menu(_) | Phase::Screen { .. } | Phase::Timeline { .. } => {}
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
        self.frames += 1;
        self.run_frames += 1;
        for id in std::mem::take(&mut self.game.sounds.start) {
            self.play_sound(id);
        }
        match self.game.exit {
            Some(Exit::Won) => {
                self.records.add(self.level as i32 + 1, self.run_frames, self.run_deaths);
                self.run_frames = 0;
                self.run_deaths = 0;
                // after the last level the picture 1002, then the first
                // level again (CODE 7 0x9a4)
                self.level = (self.level + 1) % self.data.levels.len();
                self.end_picture = self.level == 0;
                self.game.next_level(self.data.levels[self.level].clone());
                self.message(Shown::Perfect, sound::PERFECT);
            }
            Some(exit @ (Exit::Died | Exit::SelfDestructed | Exit::GameOver)) => {
                // rule 1: a death costs nothing but the way back from the
                // last checkpoint; the lives never run out
                self.deaths += 1;
                self.run_deaths += 1;
                self.game.player.lives = scout_core::game::LIVES;
                self.game.restart();
                if exit == Exit::SelfDestructed {
                    self.message(Shown::Aborted, sound::ABORTED);
                } else {
                    self.message(Shown::TryAgain, sound::TRY_AGAIN);
                }
            }
            None => {
                if self.rewind.due(self.frames) {
                    let picture = self.snapshot_picture();
                    let info = format!(
                        "Level {} {}   {}   {} {}",
                        self.level + 1,
                        menu::level_name(self.data, self.level),
                        clock(self.run_frames, FRAMES_PER_SECOND),
                        self.deaths,
                        if self.deaths == 1 { "death" } else { "deaths" }
                    );
                    self.rewind.take(self.frames, self.snap(), picture, info);
                }
            }
        }
    }

    /// The game screen at half its size as PNG, for the timeline.
    fn snapshot_picture(&self) -> Vec<u8> {
        self.rgb(&draw(self.data, &self.game, View::Screen)).half().png()
    }

    /// A canvas in the game's colours.
    fn rgb(&self, image: &Canvas) -> Png {
        let mut p = Png::new(image.width, image.height);
        image.to_rgb24(&self.data.palettes[self.palette].palette, &mut p.rgb, image.width * 3);
        p
    }

    /// Level `level`'s start view (the level selection's preview).
    fn preview(&self, level: usize) -> Png {
        self.previews
            .borrow_mut()
            .entry(level)
            .or_insert_with(|| {
                let mut game = Game::new(self.data.levels[level].clone());
                game.skip_intro();
                self.rgb(&draw(self.data, &game, View::Screen))
            })
            .clone()
    }

    /// What the window shows now.
    fn picture(&self, font: &SystemFont) -> Picture {
        let data = self.data;
        let palette = &data.palettes[self.palette].palette;
        let look = Look { data, font };
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
                let levels: Vec<_> = (0..data.levels.len()).map(|k| (menu::level_name(data, k), self.records.done(k as i32 + 1))).collect();
                let chosen = m.chosen_level();
                let details = chosen.map(|k| self.records.lines(k as i32 + 1)).unwrap_or_default();
                let preview = chosen.map(|k| self.preview(k));
                let (picture, lines) = m.draw(&look, &self.settings, self.paused.as_ref(), &levels, &details, preview.as_ref());
                screen(picture, lines)
            }
            Phase::Timeline { view, .. } => {
                let mut s = render::Screen::black();
                let chosen = view.chosen.and_then(|k| self.rewind.get(k)).and_then(|n| Png::from_png(&n.picture));
                let info = view.chosen.map(|k| self.rewind.info(k)).unwrap_or_default();
                let lines = view.draw(&look, &mut s, SCREEN, &self.rewind.tree(), self.rewind.current(), &info, |s, r| {
                    if let Some(p) = &chosen {
                        crate::theme::scaled_into(s, r, p);
                    }
                });
                screen(s, lines)
            }
            Phase::Screen { message, .. } | Phase::Quiet(message) => {
                canvas(message_canvas(data, self.level, self.deaths, *message))
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

/// A point of the window on the 640 × 480 screen.
fn screen_point(canvas: &WindowCanvas, x: f32, y: f32) -> (i32, i32) {
    let (mut fx, mut fy) = (0.0f32, 0.0f32);
    // SAFETY: the renderer is alive; the out pointers are valid
    unsafe {
        sdl3::sys::render::SDL_RenderCoordinatesFromWindow(canvas.raw(), x, y, &mut fx, &mut fy);
    }
    (fx.floor() as i32, fy.floor() as i32)
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
            if app.event(event, &mut canvas) == Flow::Quit {
                return Ok(());
            }
        }

        app.music.fill();
        let now = Instant::now();
        while next_frame <= now {
            next_frame += frame_time;
            app.frame(&events.keyboard_state());
        }

        let picture = app.picture(&font);
        let (w, h) = (picture.width as u32, picture.height as u32);
        let mut texture = upload_texture(&creator, PixelFormat::RGB24, (w, h), &picture.rgb)?;
        texture.set_scale_mode(ScaleMode::Nearest);
        let mode = if app.settings.integer_scaling { SDL_LOGICAL_PRESENTATION_INTEGER_SCALE } else { SDL_LOGICAL_PRESENTATION_LETTERBOX };
        canvas.set_logical_size(w, h, mode)?;

        // the system font at the resolution of the window
        if picture.lines.is_empty() {
            text = None;
        } else {
            let (ow, oh) = canvas.output_size()?;
            // the picture's size in the window: whole multiples, or as
            // large as it fits
            let scale = (ow as f32 / w as f32).min(oh as f32 / h as f32);
            let scale = if app.settings.integer_scaling { scale.floor().max(1.0) } else { scale.max(0.25) };
            let size = ((w as f32 * scale) as u32, (h as f32 * scale) as u32);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A game kept as JSON plays on exactly as the one it was made from.
    #[test]
    fn a_kept_game_plays_on_the_same() {
        let data = GameData::from_pack(crate::BUNDLED).unwrap();
        let mut game = Game::new(data.levels[2].clone());
        game.skip_intro();
        let keys = [Key::Move(Dir::Right), Key::Move(Dir::Down), Key::Fire, Key::Move(Dir::Left), Key::Use];
        for f in 0..400 {
            game.tick(keys[f / 37 % keys.len()]);
        }
        let snap = Snap { level: 2, game, no_walls: false, deaths: 1, run_frames: 400, run_deaths: 1, frames: 400, playing: true };
        let json = serde_json::to_string(&snap).unwrap();
        let mut copy: Snap = serde_json::from_str(&json).unwrap();
        let mut game = snap.game;
        for f in 0..600 {
            let k = keys[(f / 23 + 1) % keys.len()];
            game.tick(k);
            copy.game.tick(k);
        }
        assert_eq!(serde_json::to_string(&game).unwrap(), serde_json::to_string(&copy.game).unwrap());
        assert_eq!((game.player.x, game.player.y), (copy.game.player.x, copy.game.player.y));
    }
}
