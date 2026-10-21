//! Title, menu and the dialogs of the original (Game → Play, Resume,
//! Setting, Enter Password; About; File → Quit), drawn in the style of the
//! game: marble boxes and block letters instead of the Mac menu bar and
//! dialogs. Hints and key names are in the system font.

use scout_core::Dir;
use scout_data::GameData;
use sdl3::keyboard::Scancode;

use crate::render::{self, Canvas, Screen};
use crate::settings::{KEY_NAMES, Settings};
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
    Settings {
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

/// Rows of the settings page: three switches, the six keys, back.
const SETTING_ROWS: usize = 3 + 6 + 1;

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
            Page::Settings { waiting: true } => {
                if code != Scancode::Escape {
                    settings.set_key(self.selected - 3, code);
                    settings.save();
                }
                self.page = Page::Settings { waiting: false };
                Action::None
            }
            Page::Settings { waiting: false } => match code {
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
                    let row = self.selected;
                    match row {
                        0 => settings.music = !settings.music,
                        1 => settings.sounds = !settings.sounds,
                        2 => settings.fixed_music = !settings.fixed_music,
                        3..=8 => {
                            if matches!(
                                code,
                                Scancode::Return | Scancode::KpEnter | Scancode::Space
                            ) {
                                self.page = Page::Settings { waiting: true };
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
                self.open(Page::Settings { waiting: false });
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
            _ if matches!(self.page, Page::Settings { .. }) => Screen::black(),
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
            Page::Settings { waiting } => {
                let (bx, by, cols, rows) = (ax, ay + 6, 16, 11);
                canvas.marble_box(data, bx, by, cols, rows);
                canvas.block_text(&data.font, bx + 128, by + 28, "SETTINGS");
                let on = |b: bool| if b { "ON" } else { "OFF" };
                let small = &data.small_font;
                let value_x = bx + 304;
                for row in 0..SETTING_ROWS {
                    let y = by + 72 + 24 * row as i32;
                    let label = match row {
                        0 => "MUSIC",
                        1 => "SOUND EFFECTS",
                        2 => "MUSIC PLAYER",
                        3..=8 => KEY_NAMES[row - 3],
                        _ => "BACK",
                    };
                    canvas.block_text(small, bx + 64, y, label);
                    match row {
                        0 => canvas.block_text(small, value_x, y, on(settings.music)),
                        1 => canvas.block_text(small, value_x, y, on(settings.sounds)),
                        2 => canvas.block_text(
                            small,
                            value_x,
                            y,
                            if settings.fixed_music {
                                "CORRECTED"
                            } else {
                                "ORIGINAL"
                            },
                        ),
                        3..=8 => {
                            let text = if *waiting && row == self.selected {
                                "PRESS A KEY".to_string()
                            } else {
                                key_label(settings.keys[row - 3])
                            };
                            if text.chars().count() <= 11 && render::in_block_font(&text) {
                                canvas.block_text(small, value_x, y, &text);
                            } else {
                                // a key whose name the block letters cannot show
                                lines.push(
                                    Line::new(
                                        value_x as f32,
                                        y as f32 - 1.0,
                                        settings.keys[row - 3].name(),
                                    )
                                    .color([0xFF, 0xFF, 0xFF]),
                                );
                            }
                        }
                        _ => {}
                    }
                    if row == self.selected {
                        canvas.draw_object_half(data, cursor(), bx + 40, y);
                    }
                }
                screen.overlay(&canvas, game_palette, (bx, by, cols * 32, rows * 32));
                let hint = if *waiting {
                    "Press the new key    Esc keep the old one"
                } else {
                    "↑ ↓ choose    Return change    Esc back"
                };
                lines.push(Line::new(320.0, HINT_Y, hint).centered().color(DIM));
                lines.push(
                    Line::new(
                        320.0,
                        18.0,
                        "Esc blows yourself up during the game (as in the original)",
                    )
                    .centered()
                    .color(DIM),
                );
            }
            Page::Password { text, wrong } => {
                let (cols, rows) = (12, 5);
                let (bx, by) = (
                    ax + (aw as i32 - cols as i32 * 32) / 2,
                    ay + (ah as i32 - rows as i32 * 32) / 2,
                );
                canvas.marble_box(data, bx, by, cols, rows);
                canvas.block_text(&data.font, bx + 48, by + 24, "PASSWORD:");
                let entry = if *wrong {
                    "WRONG!".to_string()
                } else {
                    format!("{:.<6}", text)
                };
                canvas.block_text(&data.font, bx + 96, by + 96, &entry);
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
