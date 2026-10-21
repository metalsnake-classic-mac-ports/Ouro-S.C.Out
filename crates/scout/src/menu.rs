//! Title, menu and the dialogs of the original (Game → Play, Resume,
//! Setting, Enter Password; About; File → Quit), drawn in the style of the
//! game: marble boxes and block letters instead of the Mac menu bar and
//! dialogs. Hints and key names are in the system font.

use scout_core::Dir;
use scout_data::GameData;
use sdl3::keyboard::Scancode;

use crate::render::{self, Canvas, Screen};
use crate::settings::Settings;
use crate::text::Line;

/// The picture of the title (PICT 1000); after a lost game the original
/// leaves the game over picture (1001) instead.
pub const TITLE: i16 = 1000;
pub const GAME_OVER_PICTURE: i16 = 1001;
/// After the last level (PICT 1002).
pub const END_PICTURE: i16 = 1002;
/// Before the title (PICT 2000).
pub const LOGO: i16 = 2000;
const ABOUT: i16 = 256;
/// The cheat code of the password dialog (CODE 2 0x4a8).
const NOWALL: &str = "NOWALL";
const HINT_Y: f32 = 446.0;
const DIM: [u8; 3] = [0x99, 0x99, 0x99];

/// What is behind the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background {
    Picture(i16),
    /// The game screen with the PAUSE box (Command + P).
    Paused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    /// Only the background; any key opens the menu.
    Closed,
    Main,
    Settings,
    /// The keys, a part of the settings; `waiting` for the new key.
    Keys {
        waiting: bool,
    },
    Password {
        text: String,
        wrong: bool,
    },
    About,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Resume,
    Play,
    Password,
    Settings,
    About,
    Quit,
}

impl Item {
    fn label(self) -> &'static str {
        match self {
            Item::Resume => "RESUME",
            Item::Play => "PLAY",
            Item::Password => "PASSWORD",
            Item::Settings => "SETTINGS",
            Item::About => "ABOUT",
            Item::Quit => "QUIT",
        }
    }
}

/// What the menu asks the program to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Resume,
    /// A new game from the first level.
    Play,
    /// A new game from this level (from 0), by its code.
    Level(usize),
    /// The code NOWALL was entered.
    NoWalls,
    /// Music, sound or the music player changed.
    SettingsChanged,
    Quit,
}

pub struct Menu {
    pub background: Background,
    pub page: Page,
    pub selected: usize,
    /// A paused game can be resumed (Game → Resume is enabled).
    pub resumable: bool,
}

/// Rows of the settings page: three switches, the keys, back.
const SETTING_ROWS: usize = 4;
/// The row of the keys on the settings page.
const KEYS_ROW: usize = 2;
/// Rows of the keys page: the keys, back.
const KEY_ROWS: usize = KEY_LABELS.len() + 1;

impl Menu {
    pub fn new(background: Background, resumable: bool) -> Self {
        Self {
            background,
            page: Page::Closed,
            selected: 0,
            resumable,
        }
    }

    pub fn items(&self) -> Vec<Item> {
        let mut items = vec![];
        if self.resumable {
            items.push(Item::Resume);
        }
        items.extend([
            Item::Play,
            Item::Password,
            Item::Settings,
            Item::About,
            Item::Quit,
        ]);
        items
    }

    pub fn open(&mut self, page: Page) {
        self.page = page;
        self.selected = 0;
    }

    /// A key on the menu (not the Command / Ctrl shortcuts).
    pub fn key(&mut self, code: Scancode, settings: &mut Settings, data: &GameData) -> Action {
        match &mut self.page {
            Page::Closed => {
                self.open(Page::Main);
                Action::None
            }
            Page::Main => {
                let items = self.items();
                match code {
                    Scancode::Up => self.selected = (self.selected + items.len() - 1) % items.len(),
                    Scancode::Down | Scancode::Tab => {
                        self.selected = (self.selected + 1) % items.len()
                    }
                    Scancode::Escape => self.page = Page::Closed,
                    Scancode::Return | Scancode::KpEnter | Scancode::Space => {
                        return self.choose(items[self.selected]);
                    }
                    _ => {}
                }
                Action::None
            }
            Page::Settings => match code {
                Scancode::Up => {
                    self.selected = (self.selected + SETTING_ROWS - 1) % SETTING_ROWS;
                    Action::None
                }
                Scancode::Down | Scancode::Tab => {
                    self.selected = (self.selected + 1) % SETTING_ROWS;
                    Action::None
                }
                Scancode::Escape => self.back(Item::Settings),
                Scancode::Return
                | Scancode::KpEnter
                | Scancode::Space
                | Scancode::Left
                | Scancode::Right => {
                    match self.selected {
                        // off, as in the original, fixed
                        0 => {
                            let now = music_choice(settings);
                            let next = if code == Scancode::Left {
                                now + 2
                            } else {
                                now + 1
                            } % 3;
                            settings.music = next != 0;
                            if next != 0 {
                                settings.fixed_music = next == 2;
                            }
                        }
                        1 => settings.sounds = !settings.sounds,
                        KEYS_ROW => {
                            if !matches!(code, Scancode::Left | Scancode::Right) {
                                self.open(Page::Keys { waiting: false });
                            }
                            return Action::None;
                        }
                        _ => return self.back(Item::Settings),
                    }
                    settings.save();
                    Action::SettingsChanged
                }
                _ => Action::None,
            },
            Page::Keys { waiting: true } => {
                // any key, Esc included (it is the default for self destruct)
                settings.set_key(self.selected, code);
                settings.save();
                self.page = Page::Keys { waiting: false };
                Action::None
            }
            Page::Keys { waiting: false } => {
                match code {
                    Scancode::Up => self.selected = (self.selected + KEY_ROWS - 1) % KEY_ROWS,
                    Scancode::Down | Scancode::Tab => {
                        self.selected = (self.selected + 1) % KEY_ROWS
                    }
                    Scancode::Escape => self.back_to_settings(),
                    Scancode::Return | Scancode::KpEnter | Scancode::Space => {
                        if self.selected < KEY_LABELS.len() {
                            self.page = Page::Keys { waiting: true };
                        } else {
                            self.back_to_settings();
                        }
                    }
                    _ => {}
                }
                Action::None
            }
            Page::Password { text, wrong } => {
                if *wrong {
                    *wrong = false;
                    text.clear();
                    return Action::None;
                }
                match code {
                    Scancode::Escape => self.back(Item::Password),
                    Scancode::Backspace => {
                        text.pop();
                        Action::None
                    }
                    Scancode::Return | Scancode::KpEnter => {
                        let code = text.clone();
                        self.password(&code, data)
                    }
                    _ => {
                        let name = code.name();
                        let mut chars = name.chars();
                        if let (Some(c), None) = (chars.next(), chars.next())
                            && c.is_ascii_alphanumeric()
                            && text.len() < 6
                        {
                            text.push(c.to_ascii_uppercase());
                        }
                        Action::None
                    }
                }
            }
            Page::About => self.back(Item::About),
        }
    }

    /// From the keys back to the settings, with KEYS selected.
    fn back_to_settings(&mut self) {
        self.page = Page::Settings;
        self.selected = KEYS_ROW;
    }

    /// Back to the main menu with `item` selected.
    fn back(&mut self, item: Item) -> Action {
        self.page = Page::Main;
        self.selected = self.items().iter().position(|&i| i == item).unwrap_or(0);
        Action::None
    }

    pub fn choose(&mut self, item: Item) -> Action {
        match item {
            Item::Resume => Action::Resume,
            Item::Play => Action::Play,
            Item::Password => {
                self.open(Page::Password {
                    text: String::new(),
                    wrong: false,
                });
                Action::None
            }
            Item::Settings => {
                self.open(Page::Settings);
                Action::None
            }
            Item::About => {
                self.page = Page::About;
                Action::None
            }
            Item::Quit => Action::Quit,
        }
    }

    /// OK in the password dialog (CODE 2 0x46e): a level code starts its
    /// level; NOWALL switches the cheat on (and, like the original, is
    /// still a wrong code).
    fn password(&mut self, code: &str, data: &GameData) -> Action {
        if let Some(level) = Self::level_of(data, code) {
            return Action::Level(level);
        }
        let action = if code == NOWALL {
            Action::NoWalls
        } else {
            Action::None
        };
        self.page = Page::Password {
            text: code.to_string(),
            wrong: true,
        };
        action
    }

    /// The level (from 0) of an entered code (CODE 2 0x63e: the code of
    /// entry i starts level i + 1; the first level has none).
    pub fn level_of(data: &GameData, code: &str) -> Option<usize> {
        data.level_codes
            .iter()
            .position(|c| c.eq_ignore_ascii_case(code))
            .map(|i| i + 1)
    }

    /// The menu over its background, and the lines in the system font.
    pub fn draw(
        &self,
        data: &GameData,
        settings: &Settings,
        paused: Option<&Canvas>,
    ) -> (Screen, Vec<Line>) {
        let palette = &data.palettes[0].palette;
        let game_palette = data
            .palettes
            .iter()
            .find(|p| p.name == "game")
            .map_or(palette, |p| &p.palette);
        let mut screen = match (self.background, paused) {
            // the settings cover the whole picture area
            _ if matches!(self.page, Page::Settings | Page::Keys { .. }) => Screen::black(),
            (Background::Paused, Some(canvas)) => {
                let mut canvas = canvas.clone();
                render::pause_box(data, &mut canvas);
                Screen::from_canvas(&canvas, game_palette)
            }
            (Background::Picture(id), _) => {
                let mut screen = Screen::black();
                screen.picture(data, id, render::PICTURE_AREA);
                screen
            }
            (Background::Paused, None) => Screen::black(),
        };
        let mut lines = vec![];
        let mut canvas = Canvas::new(render::SCREEN_WIDTH, render::SCREEN_HEIGHT, 0);
        let (ax, ay, aw, ah) = render::PICTURE_AREA;
        let (ax, ay) = (ax as i32, ay as i32);
        match &self.page {
            Page::Closed => {
                lines.push(
                    Line::new(320.0, HINT_Y, "Press a key for the menu")
                        .centered()
                        .color(DIM),
                );
            }
            Page::Main => {
                let items = self.items();
                let rows = (24 + items.len() * 40 + 8).div_ceil(32);
                let (bx, by) = (
                    ax + (aw as i32 - 12 * 32) / 2,
                    ay + (ah as i32 - rows as i32 * 32) / 2,
                );
                canvas.marble_box(data, bx, by, 12, rows);
                let top = by + (rows as i32 * 32 - (items.len() as i32 * 40 - 8)) / 2;
                for (i, item) in items.iter().enumerate() {
                    let y = top + 40 * i as i32;
                    canvas.block_text(&data.font, bx + 80, y, item.label());
                    if i == self.selected {
                        canvas.draw_object(data, cursor(), bx + 32, y);
                    }
                }
                screen.overlay(&canvas, game_palette, (bx, by, 12 * 32, rows * 32));
                lines.push(
                    Line::new(320.0, HINT_Y, "↑ ↓ choose    Return select    Esc close")
                        .centered()
                        .color(DIM),
                );
            }
            Page::Settings => {
                let on = |b: bool| if b { "ON" } else { "OFF" };
                let music = ["OFF", "ORIGINAL", "FIXED"][music_choice(settings)];
                let rows = [
                    (
                        "MUSIC",
                        Value::Text(music.into()),
                        "Music off, as in the original (with the bugs of its player) or fixed",
                    ),
                    (
                        "SFX",
                        Value::Text(on(settings.sounds).into()),
                        "Sound effects on or off",
                    ),
                    ("KEYS", Value::None, "The keys of the game"),
                    ("BACK", Value::None, "Back to the menu"),
                ];
                list_page(
                    data,
                    &mut canvas,
                    &mut lines,
                    "SETTINGS",
                    &rows,
                    self.selected,
                    (48, 288),
                );
                screen.overlay(&canvas, game_palette, LIST_BOX);
                lines.push(
                    Line::new(320.0, HINT_Y, "↑ ↓ choose    Return change    Esc back")
                        .centered()
                        .color(DIM),
                );
            }
            Page::Keys { waiting } => {
                let mut rows: Vec<(&str, Value, &str)> = KEY_LABELS
                    .iter()
                    .zip(settings.keys)
                    .enumerate()
                    .map(|(i, ((label, about), key))| {
                        let value = if *waiting && i == self.selected {
                            Value::Text("?".into())
                        } else {
                            Value::Key(key)
                        };
                        (*label, value, *about)
                    })
                    .collect();
                rows.push(("BACK", Value::None, "Back to the settings"));
                list_page(
                    data,
                    &mut canvas,
                    &mut lines,
                    "KEYS",
                    &rows,
                    self.selected,
                    (38, 352),
                );
                screen.overlay(&canvas, game_palette, LIST_BOX);
                let hint = if *waiting {
                    "Press the new key"
                } else {
                    "↑ ↓ choose    Return change    Esc back"
                };
                lines.push(Line::new(320.0, HINT_Y, hint).centered().color(DIM));
            }
            Page::Password { text, wrong } => {
                let (cols, rows) = (12, 6);
                let (bx, by) = (
                    ax + (aw as i32 - cols as i32 * 32) / 2,
                    ay + (ah as i32 - rows as i32 * 32) / 2,
                );
                canvas.marble_box(data, bx, by, cols, rows);
                canvas.block_text(&data.font, bx + 48, by + 40, "PASSWORD:");
                let entry = if *wrong {
                    "WRONG!".to_string()
                } else {
                    format!("{:.<6}", text)
                };
                canvas.block_text(&data.font, bx + 96, by + 112, &entry);
                screen.overlay(&canvas, game_palette, (bx, by, cols * 32, rows * 32));
                let hint = if *wrong {
                    "Wrong password    any key: try again"
                } else {
                    "Type the code of a level    Return OK    Esc cancel"
                };
                lines.push(Line::new(320.0, HINT_Y, hint).centered().color(DIM));
            }
            Page::About => {
                let (w, h) = Screen::picture_size(data, ABOUT);
                let x = ax as usize + (aw - w) / 2;
                let y = ay as usize + (ah - h) / 2;
                screen.picture(data, ABOUT, (x, y, w, h));
                lines.push(
                    Line::new(320.0, HINT_Y, "Press a key")
                        .centered()
                        .color(DIM),
                );
            }
        }
        (screen, lines)
    }
}

/// The box of the settings and the keys: the width of the screen (they
/// are shown on black), room for the hints below.
const LIST_BOX: (i32, i32, usize, usize) = (0, 8, 20 * 32, 13 * 32);
/// Where the description of the selected row goes.
const ABOUT_Y: f32 = 426.0;

/// The labels of the keys on the keys page (at most 8 letters) and what
/// they do.
const KEY_LABELS: [(&str, &str); 7] = [
    ("RIGHT", "Move right"),
    ("LEFT", "Move left"),
    ("UP", "Move up"),
    ("DOWN", "Move down"),
    ("GET-DROP", "Take, drop or swap an object; use a teleporter"),
    ("FIRE", "Fire a missile"),
    (
        "ABORT",
        "Blow yourself up, the level is ABORTED (Esc in the original)",
    ),
];

/// The music row: 0 off, 1 as in the original, 2 fixed.
fn music_choice(settings: &Settings) -> usize {
    match (settings.music, settings.fixed_music) {
        (false, _) => 0,
        (true, false) => 1,
        (true, true) => 2,
    }
}

/// The value column of a row on a list page.
enum Value {
    None,
    Text(String),
    Key(Scancode),
}

/// A page of the settings: a title and rows of block letters with a value
/// column, the selected row marked by the player and described below in
/// the system font. `(pitch, value_x)`: rows apart, x of the values.
fn list_page(
    data: &GameData,
    canvas: &mut Canvas,
    lines: &mut Vec<Line>,
    title: &str,
    rows: &[(&str, Value, &str)],
    selected: usize,
    (pitch, value_x): (i32, i32),
) {
    let (bx, by, w, h) = LIST_BOX;
    canvas.marble_box(data, bx, by, w / 32, h / 32);
    let title_x = bx + (w as i32 - 32 * title.len() as i32) / 2;
    canvas.block_text(&data.font, title_x, by + 28, title);
    let font = &data.font;
    let (label_x, value_x) = (bx + 72, bx + value_x);
    for (row, (label, value, about)) in rows.iter().enumerate() {
        let y = by + 84 + pitch * row as i32;
        canvas.block_text(font, label_x, y, label);
        match value {
            Value::None => {}
            Value::Text(text) => canvas.block_text(font, value_x, y, text),
            Value::Key(key) => {
                let text = key_label(*key);
                if text.chars().count() <= 8 && render::in_block_font(&text) {
                    canvas.block_text(font, value_x, y, &text);
                } else {
                    // a key whose name the block letters cannot show
                    let mut line = Line::new(value_x as f32, y as f32 + 3.0, key.name())
                        .color([0xFF, 0xFF, 0xFF]);
                    line.size = 24.0;
                    lines.push(line);
                }
            }
        }
        if row == selected {
            canvas.draw_object(data, cursor(), bx + 32, y);
            lines.push(Line::new(320.0, ABOUT_Y, *about).centered());
        }
    }
}

/// The name of a key for the block letters: upper case, "Keypad" as "KP".
fn key_label(code: Scancode) -> String {
    code.name().to_uppercase().replace("KEYPAD", "KP")
}

/// The player facing right, as the menu cursor.
fn cursor() -> usize {
    4 * Dir::Right as usize + Dir::Right as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_is_offered_only_for_a_paused_game() {
        assert_eq!(
            Menu::new(Background::Picture(TITLE), false).items()[0],
            Item::Play
        );
        assert_eq!(Menu::new(Background::Paused, true).items()[0], Item::Resume);
    }
}
