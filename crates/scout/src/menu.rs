//! The title and the menus: the Ouro ports' shared screens (ouro-ui's
//! `menu`, `levels`, `options`, `keys`) in S.C.Out's look
//! ([`crate::theme::Look`]). They replace the original's menu bar and
//! dialogs (Game → Play, Resume, Setting, Enter Password; About; File →
//! Quit) by the ports' rules: Continue or Resume first, a level selection
//! instead of the password dialog (rule 2: no passwords; its cheat NOWALL
//! is typed on the main menu instead), the settings and keys on modern
//! pages (rule 9). The title, the About box and the paused game's PAUSE
//! box stay the original's.

use ouro_ui::{Ink, Key, Rect};
use scout_data::GameData;
use sdl3::keyboard::Scancode;

use crate::render::{self, Canvas, Screen};
use crate::settings::{self, Settings};
use crate::text::Line;
use crate::theme::Look;

/// The picture of the title (PICT 1000). After a lost game the original
/// leaves the game over picture (1001) instead; the port has no game over.
pub const TITLE: i16 = 1000;
/// After the last level (PICT 1002).
pub const END_PICTURE: i16 = 1002;
/// Before the title (PICT 2000).
pub const LOGO: i16 = 2000;
const ABOUT: i16 = 256;
/// The cheat code of the original's password dialog (CODE 2 0x4a8).
const NOWALL: &str = "NOWALL";
const HINT_Y: f32 = 446.0;
const DIM: [u8; 3] = [0x99, 0x99, 0x99];
const SCREEN: Rect = Rect::new(0, 0, render::SCREEN_WIDTH as i32, render::SCREEN_HEIGHT as i32);

/// What is behind the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background {
    Picture(i16),
    /// The game screen with the PAUSE box (Command + P).
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// Only the background; any key opens the menu.
    Closed,
    Main,
    Levels,
    Options,
    Keys,
    About,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Resume,
    Continue,
    Timeline,
    NewGame,
    Levels,
    Options,
    About,
    Quit,
}

impl Item {
    fn label(self) -> &'static str {
        match self {
            Item::Resume => "Resume",
            Item::Continue => "Continue",
            Item::Timeline => "Timeline",
            Item::NewGame => "New Game",
            Item::Levels => "Levels",
            Item::Options => "Options",
            Item::About => "About S.C.Out",
            Item::Quit => "Quit",
        }
    }
}

/// The rows of the options page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Opt {
    Music,
    Sounds,
    Scaling,
    Fullscreen,
    Keys,
    Defaults,
}

const OPTIONS: [Opt; 6] = [Opt::Music, Opt::Sounds, Opt::Scaling, Opt::Fullscreen, Opt::Keys, Opt::Defaults];

/// What the menu asks the program to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Resume,
    Continue,
    Timeline,
    /// A new game from the first level.
    NewGame,
    /// A game from this level (from 0), chosen in the level selection.
    Level(usize),
    /// The code NOWALL was typed.
    NoWalls,
    /// A setting changed (saved): apply it.
    SettingsChanged,
    Quit,
}

pub struct Menu {
    pub background: Background,
    pub page: Page,
    /// A paused game can be resumed.
    pub resumable: bool,
    /// A game was kept when the program or the game was left.
    pub saved: bool,
    main: ouro_ui::menu::Menu,
    levels: ouro_ui::levels::Levels,
    options: ouro_ui::options::Options,
    keys: ouro_ui::keys::Keys,
    /// The letters typed on the main page (for NOWALL).
    typed: String,
}

/// The labels of the keys on the keys page and what they do.
const KEY_LABELS: [(&str, &str); settings::KEYS] = [
    ("Right", "Move right"),
    ("Left", "Move left"),
    ("Up", "Move up"),
    ("Down", "Move down"),
    ("Get / Drop", "Take, drop or swap an object; use a teleporter"),
    ("Fire", "Fire a missile"),
    ("Self Destruct", "Blow yourself up: the level is ABORTED (the original: Esc)"),
];
const KEY_NOTES: [&str; 1] = ["Only one key counts at a time, as in the original"];

/// The main box: over the title's picture area, centred.
fn main_area(rows: usize) -> Rect {
    let (ax, ay, aw, ah) = render::PICTURE_AREA;
    let h = ouro_ui::menu::height(rows, false);
    let w = 240;
    let (left, top) = (ax as i32 + (aw as i32 - w) / 2, ay as i32 + (ah as i32 - h) / 2);
    Rect::new(left, top, left + w, top + h)
}

/// The shared screens' key for a key of the keyboard.
pub fn ui_key(code: Scancode) -> Option<Key> {
    Some(match code {
        Scancode::Up => Key::Up,
        Scancode::Down | Scancode::Tab => Key::Down,
        Scancode::Left => Key::Left,
        Scancode::Right => Key::Right,
        Scancode::PageUp => Key::PageUp,
        Scancode::PageDown => Key::PageDown,
        Scancode::Home => Key::Home,
        Scancode::End => Key::End,
        Scancode::Return | Scancode::KpEnter => Key::Accept,
        Scancode::Escape => Key::Back,
        Scancode::Space => Key::Toggle,
        _ => return None,
    })
}

/// The levels' names: the first is START (it has no code), the others are
/// named after their code (rule 3; the original names none).
pub fn level_name(data: &GameData, level: usize) -> String {
    match level.checked_sub(1) {
        None => "START".into(),
        Some(k) => data.level_codes.get(k).cloned().unwrap_or_else(|| format!("LEVEL {}", level + 1)),
    }
}

impl Menu {
    pub fn new(background: Background, resumable: bool) -> Self {
        Self {
            background,
            page: Page::Closed,
            resumable,
            saved: false,
            main: Default::default(),
            levels: Default::default(),
            options: Default::default(),
            keys: Default::default(),
            typed: String::new(),
        }
    }

    pub fn items(&self) -> Vec<Item> {
        let mut items = vec![];
        if self.resumable {
            items.extend([Item::Resume, Item::Timeline]);
        } else if self.saved {
            items.push(Item::Continue);
        }
        items.extend([Item::NewGame, Item::Levels, Item::Options, Item::About, Item::Quit]);
        items
    }

    pub fn open(&mut self, page: Page) {
        self.page = page;
        match page {
            Page::Main => self.main.select(0),
            Page::Options => self.options = Default::default(),
            Page::Keys => self.keys = Default::default(),
            _ => {}
        }
    }

    /// Open the level selection on `level` (from 0).
    pub fn open_levels(&mut self, level: usize) {
        self.page = Page::Levels;
        self.levels.select(level, SCREEN);
    }

    /// Waiting for a new key on the keys page.
    pub fn waiting(&self) -> bool {
        self.page == Page::Keys && self.keys.waiting
    }

    /// A key on the menu (not the Command / Ctrl shortcuts).
    pub fn key(&mut self, code: Scancode, settings: &mut Settings, levels: usize) -> Action {
        match self.page {
            Page::Closed => {
                self.open(Page::Main);
                Action::None
            }
            Page::About => self.back(Item::About),
            Page::Main => {
                if let Some(c) = crate::menu::key_char(code).filter(char::is_ascii_alphabetic) {
                    self.typed.push(c.to_ascii_uppercase());
                    if self.typed.len() > NOWALL.len() {
                        self.typed.remove(0);
                    }
                    if self.typed == NOWALL {
                        self.typed.clear();
                        return Action::NoWalls;
                    }
                    return Action::None;
                }
                let items = self.items();
                let entries: Vec<_> = items.iter().map(|_| ouro_ui::menu::Entry::new("")).collect();
                match ui_key(code).and_then(|k| self.main.key(k, &entries)) {
                    Some(ouro_ui::menu::Choice::Take(k)) => self.choose(items[k]),
                    Some(ouro_ui::menu::Choice::Back) if self.resumable => Action::Resume,
                    Some(ouro_ui::menu::Choice::Back) => {
                        self.page = Page::Closed;
                        Action::None
                    }
                    None => Action::None,
                }
            }
            Page::Levels => match ui_key(code).and_then(|k| self.levels.key(k, levels, SCREEN)) {
                Some(ouro_ui::levels::Choice::Take(k)) => Action::Level(k),
                Some(ouro_ui::levels::Choice::Back) => self.back(Item::Levels),
                None => Action::None,
            },
            Page::Options => match ui_key(code).and_then(|k| self.options.key(k, &option_rows(settings))) {
                Some(choice) => self.options_choice(choice, settings),
                None => Action::None,
            },
            Page::Keys if self.keys.waiting => {
                // Escape keeps the old key (Defaults gives it back to self
                // destruct); the shortcuts' modifiers and F11 cannot be taken
                if code != Scancode::Escape && !settings::reserved(code) {
                    settings.set_key(self.keys.selected, code);
                    settings.save();
                }
                self.keys.done();
                Action::None
            }
            Page::Keys => match ui_key(code).and_then(|k| self.keys.key(k, settings::KEYS)) {
                Some(choice) => self.keys_choice(choice, settings),
                None => Action::None,
            },
        }
    }

    /// The mouse at (x, y) of the 640 × 480 screen; `click`: its button
    /// was pressed.
    pub fn mouse(&mut self, x: i32, y: i32, click: bool, settings: &mut Settings, levels: usize) -> Action {
        match self.page {
            Page::Closed | Page::About if click => self.key(Scancode::Space, settings, levels),
            Page::Closed | Page::About => Action::None,
            Page::Main => {
                let items = self.items();
                let entries: Vec<_> = items.iter().map(|_| ouro_ui::menu::Entry::new("")).collect();
                match self.main.mouse(x, y, click, &entries, main_area(items.len()), false) {
                    Some(ouro_ui::menu::Choice::Take(k)) => self.choose(items[k]),
                    _ => Action::None,
                }
            }
            Page::Levels => match self.levels.mouse(x, y, click, levels, SCREEN) {
                Some(ouro_ui::levels::Choice::Take(k)) => Action::Level(k),
                Some(ouro_ui::levels::Choice::Back) => self.back(Item::Levels),
                None => Action::None,
            },
            Page::Options => match self.options.mouse(x, y, click, &option_rows(settings), 0, SCREEN) {
                Some(choice) => self.options_choice(choice, settings),
                None => Action::None,
            },
            Page::Keys => match self.keys.mouse(x, y, click, settings::KEYS, KEY_NOTES.len(), SCREEN) {
                Some(choice) => self.keys_choice(choice, settings),
                None => Action::None,
            },
        }
    }

    /// The mouse wheel on the level list.
    pub fn wheel(&mut self, dy: f32, levels: usize) {
        if self.page == Page::Levels {
            self.levels.wheel(dy.round() as i32, levels, SCREEN);
        }
    }

    fn options_choice(&mut self, choice: ouro_ui::options::Choice, settings: &mut Settings) -> Action {
        use ouro_ui::options::Choice;
        let row = match choice {
            Choice::Back => return self.back(Item::Options),
            Choice::Change(k, _) | Choice::Take(k) => OPTIONS[k],
        };
        match row {
            Opt::Music => settings.music = !settings.music,
            Opt::Sounds => settings.sounds = !settings.sounds,
            Opt::Scaling => settings.integer_scaling = !settings.integer_scaling,
            Opt::Fullscreen => settings.fullscreen = !settings.fullscreen,
            Opt::Keys => {
                self.open(Page::Keys);
                return Action::None;
            }
            Opt::Defaults => {
                let d = Settings::default();
                *settings = Settings { keys: settings.keys, ..d };
            }
        }
        settings.save();
        Action::SettingsChanged
    }

    fn keys_choice(&mut self, choice: ouro_ui::keys::Choice, settings: &mut Settings) -> Action {
        match choice {
            // the page waits for the key now
            ouro_ui::keys::Choice::Change(_) => {}
            ouro_ui::keys::Choice::Defaults => {
                settings.keys = Settings::default().keys;
                settings.save();
            }
            ouro_ui::keys::Choice::Back => {
                self.page = Page::Options;
                self.options.selected = OPTIONS.iter().position(|&o| o == Opt::Keys).unwrap_or(0);
            }
        }
        Action::None
    }

    /// Back to the main menu with `item` chosen.
    fn back(&mut self, item: Item) -> Action {
        self.page = Page::Main;
        self.main.select(self.items().iter().position(|&i| i == item).unwrap_or(0));
        Action::None
    }

    pub fn choose(&mut self, item: Item) -> Action {
        match item {
            Item::Resume => Action::Resume,
            Item::Continue => Action::Continue,
            Item::Timeline => Action::Timeline,
            Item::NewGame => Action::NewGame,
            Item::Levels => {
                self.open_levels(0);
                Action::None
            }
            Item::Options => {
                self.open(Page::Options);
                Action::None
            }
            Item::About => {
                self.page = Page::About;
                Action::None
            }
            Item::Quit => Action::Quit,
        }
    }

    /// The level chosen in the level selection (from 0).
    pub fn chosen_level(&self) -> Option<usize> {
        (self.page == Page::Levels).then_some(self.levels.selected)
    }

    /// The menu over its background, and its lines in the system font.
    /// `levels`: each level's name and whether it was done; `details`
    /// the chosen one's records; `preview` its start view.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        look: &Look,
        settings: &Settings,
        paused: Option<&Canvas>,
        levels: &[(String, bool)],
        details: &[(String, Ink)],
        preview: Option<&ouro_play::picture::Picture>,
    ) -> (Screen, Vec<Line>) {
        let data = look.data;
        let mut screen = match (self.background, paused) {
            (Background::Paused, Some(canvas)) => {
                let mut canvas = canvas.clone();
                render::pause_box(data, &mut canvas);
                Screen::from_canvas(&canvas, &data.game_palette)
            }
            (Background::Picture(id), _) => {
                let mut screen = Screen::black();
                screen.picture(data, id, render::PICTURE_AREA);
                screen
            }
            (Background::Paused, None) => Screen::black(),
        };
        let mut lines = vec![];
        match self.page {
            Page::Closed => lines.push(hint("Press a key for the menu")),
            Page::Main => {
                let entries: Vec<_> = self.items().iter().map(|i| ouro_ui::menu::Entry::new(i.label())).collect();
                lines.extend(self.main.draw(look, &mut screen, main_area(entries.len()), None, &entries));
                lines.push(hint(if self.resumable { "↑ ↓ choose    Return select    Esc back to the game" } else { "↑ ↓ choose    Return select    Esc close" }));
            }
            Page::Levels => {
                let rows: Vec<_> = levels
                    .iter()
                    .enumerate()
                    .map(|(k, (name, done))| ouro_ui::levels::Row { number: Some(k as i32 + 1), name: name.clone(), done: *done, locked: false })
                    .collect();
                lines.extend(self.levels.draw(look, &mut screen, SCREEN, "Levels", &rows, details, |s, r| match preview {
                    Some(p) => crate::theme::scaled_into(s, r, p),
                    None => crate::theme::fill(s, r, [0; 3]),
                }));
            }
            Page::Options => lines.extend(self.options.draw(look, &mut screen, SCREEN, "Options", &option_rows(settings), &[])),
            Page::Keys => lines.extend(self.keys.draw(look, &mut screen, SCREEN, &bindings(settings), &KEY_NOTES)),
            Page::About => {
                let (ax, ay, aw, ah) = render::PICTURE_AREA;
                let (w, h) = Screen::picture_size(data, ABOUT);
                screen.picture(data, ABOUT, (ax + (aw - w) / 2, ay + (ah - h) / 2, w, h));
                lines.push(hint("Press a key"));
            }
        }
        (screen, lines)
    }
}

fn option_rows(s: &Settings) -> Vec<ouro_ui::options::Row> {
    use ouro_ui::options::{Row, on_off};
    OPTIONS
        .iter()
        .map(|o| match o {
            Opt::Music => Row::setting("Music", on_off(s.music), "The title music, on or off"),
            Opt::Sounds => Row::setting("Sound Effects", on_off(s.sounds), "The game's sounds, on or off"),
            Opt::Scaling => Row::setting(
                "Scaling",
                if s.integer_scaling { "Whole multiples (sharp)" } else { "Fill the window" },
                "The picture scaled by whole multiples, or as large as the window allows",
            ),
            Opt::Fullscreen => Row::setting("Full Screen", on_off(s.fullscreen), "The whole screen or a window (also F11 or Command / Ctrl + F)"),
            Opt::Keys => Row::action("Keys...", "The keys of the game"),
            Opt::Defaults => Row::action("Defaults", "Music, sounds, scaling and full screen as at first (the keys stay)"),
        })
        .collect()
}

fn bindings(s: &Settings) -> Vec<ouro_ui::keys::Binding> {
    KEY_LABELS
        .iter()
        .zip(s.keys)
        .map(|((action, about), key)| ouro_ui::keys::Binding { action: (*action).into(), key: key.name().to_string(), pad: String::new(), about: (*about).into() })
        .collect()
}

/// A hint at the bottom of the screen.
fn hint(text: &str) -> Line {
    Line::new(320.0, HINT_Y, text).centered().color(DIM)
}

/// The character of a key whose name is one character (letters, digits).
pub fn key_char(code: Scancode) -> Option<char> {
    let name = code.name();
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continue_and_resume() {
        assert_eq!(Menu::new(Background::Picture(TITLE), false).items()[0], Item::NewGame);
        let mut m = Menu::new(Background::Picture(TITLE), false);
        m.saved = true;
        assert_eq!(m.items()[0], Item::Continue);
        assert_eq!(&Menu::new(Background::Paused, true).items()[..2], &[Item::Resume, Item::Timeline]);
    }

    #[test]
    fn nowall_typed_on_the_main_page() {
        let mut m = Menu::new(Background::Picture(TITLE), false);
        let mut s = Settings::default();
        m.open(Page::Main);
        let mut last = Action::None;
        for c in [Scancode::X, Scancode::N, Scancode::O, Scancode::W, Scancode::A, Scancode::L, Scancode::L] {
            last = m.key(c, &mut s, 10);
        }
        assert_eq!(last, Action::NoWalls);
    }

    #[test]
    fn options_and_keys() {
        let mut m = Menu::new(Background::Picture(TITLE), false);
        let mut s = Settings::default();
        m.open(Page::Main);
        // New Game, Levels, Options
        m.key(Scancode::Down, &mut s, 10);
        m.key(Scancode::Down, &mut s, 10);
        m.key(Scancode::Return, &mut s, 10);
        assert_eq!(m.page, Page::Options);
        // Scaling off (tests never save the settings)
        m.key(Scancode::Down, &mut s, 10);
        m.key(Scancode::Down, &mut s, 10);
        assert_eq!(m.key(Scancode::Right, &mut s, 10), Action::SettingsChanged);
        assert!(!s.integer_scaling);
        m.key(Scancode::Down, &mut s, 10);
        m.key(Scancode::Down, &mut s, 10);
        m.key(Scancode::Return, &mut s, 10);
        assert_eq!(m.page, Page::Keys);
        // fire on F; Escape keeps the old key
        for _ in 0..5 {
            m.key(Scancode::Down, &mut s, 10);
        }
        m.key(Scancode::Return, &mut s, 10);
        assert!(m.waiting());
        m.key(Scancode::Escape, &mut s, 10);
        assert_eq!(s.keys[5], Scancode::Space);
        m.key(Scancode::Escape, &mut s, 10);
        assert_eq!(m.page, Page::Options);
        m.key(Scancode::Escape, &mut s, 10);
        assert_eq!(m.page, Page::Main);
    }

    #[test]
    fn levels_take_a_level() {
        let mut m = Menu::new(Background::Picture(TITLE), false);
        let mut s = Settings::default();
        m.open_levels(0);
        m.key(Scancode::Down, &mut s, 10);
        assert_eq!(m.chosen_level(), Some(1));
        assert_eq!(m.key(Scancode::Return, &mut s, 10), Action::Level(1));
        m.key(Scancode::Escape, &mut s, 10);
        assert_eq!(m.page, Page::Main);
    }
}
