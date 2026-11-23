//! The settings (Game → Setting in the original, CODE 2 0x1108): music,
//! sound effects and the six game keys, plus our own options. The original
//! keeps them in its own resource `SHAP 1` (`SHAP 0` if it is missing) and
//! writes that back into the application when it quits (CODE 5 0x16ce); we
//! keep them in a text file in the user's settings folder.

use std::path::PathBuf;

use sdl3::keyboard::Scancode;

/// The number of game keys.
pub const KEYS: usize = 7;

/// The game keys in the order of the original dialog, then self destruct
/// (fixed to Esc in the original); their names in the settings file.
pub const KEY_NAMES: [&str; KEYS] = [
    "RIGHT",
    "LEFT",
    "UP",
    "DOWN",
    "GET-DROP",
    "FIRE",
    "SELF DESTRUCT",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub music: bool,
    pub sounds: bool,
    /// Right, left, up, down, get / drop, fire, self destruct.
    pub keys: [Scancode; KEYS],
    pub fullscreen: bool,
    /// How the picture is scaled to the window (`scaling = whole | fill |
    /// original`).
    pub scaling: Scaling,
}

/// How the picture is scaled to the window or the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scaling {
    /// By the largest whole multiple that fits (sharp pixels).
    Whole,
    /// As large as fits, keeping the shape.
    Fill,
    /// Not at all: one pixel of the original is one point of the screen,
    /// in the middle (cut off if the window is smaller).
    Original,
}

impl Scaling {
    const ALL: [Scaling; 3] = [Scaling::Whole, Scaling::Fill, Scaling::Original];

    pub fn name(self) -> &'static str {
        match self {
            Scaling::Whole => "Whole multiples (sharp)",
            Scaling::Fill => "Fill the window",
            Scaling::Original => "Original size",
        }
    }

    /// `by` steps on, round.
    pub fn step(self, by: i32) -> Scaling {
        let i = Scaling::ALL.iter().position(|&s| s == self).unwrap_or(0) as i32;
        Scaling::ALL[(i + by).rem_euclid(Scaling::ALL.len() as i32) as usize]
    }

    /// The word in the settings file.
    fn word(self) -> &'static str {
        match self {
            Scaling::Whole => "whole",
            Scaling::Fill => "fill",
            Scaling::Original => "original",
        }
    }
}

impl Default for Settings {
    /// The original (1.4) has music and sound on and the keypad (6, 4, 8,
    /// 2, Enter, 5) and Esc; we start with the arrow keys, Return, Space and
    /// Esc.
    fn default() -> Self {
        Self {
            music: true,
            sounds: true,
            keys: [
                Scancode::Right,
                Scancode::Left,
                Scancode::Up,
                Scancode::Down,
                Scancode::Return,
                Scancode::Space,
                Scancode::Escape,
            ],
            fullscreen: false,
            scaling: Scaling::Whole,
        }
    }
}

const FILE: &str = "settings.txt";

/// The settings folder (SDL's preference folder, macOS:
/// `~/Library/Application Support/Ouro S.C.Out`).
const FOLDER: &str = "Ouro S.C.Out";

/// The settings folder, where the records and Continue's game are kept
/// too (none in tests: they never touch the player's files).
pub fn folder() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    pref_dir()
}

/// The folder for everything the game keeps (settings, records, the kept
/// game ...): `OURO_PREFS` if set (tests, a fresh start, someone's state
/// to look at; made if missing), else SDL's preference folder.
pub fn pref_dir() -> Option<PathBuf> {
    match std::env::var_os("OURO_PREFS") {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            std::fs::create_dir_all(&dir).ok()?;
            Some(dir)
        }
        None => sdl3::filesystem::get_pref_path("", FOLDER).ok(),
    }
}

fn path() -> Option<PathBuf> {
    Some(pref_dir()?.join(FILE))
}

impl Settings {
    /// The saved settings, or the defaults.
    pub fn load() -> Self {
        let mut settings = Self::default();
        let Some(text) = path().and_then(|p| std::fs::read_to_string(p).ok()) else {
            return settings;
        };
        for line in text.lines() {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            let (name, value) = (name.trim(), value.trim());
            let on = value == "on";
            match name {
                "music" => settings.music = on,
                "sounds" => settings.sounds = on,
                "screen" => settings.fullscreen = value == "full",
                "scaling" => settings.scaling = Scaling::ALL.into_iter().find(|s| s.word() == value).unwrap_or(Scaling::Whole),
                _ => {
                    if let Some(i) = KEY_NAMES.iter().position(|k| k.eq_ignore_ascii_case(name))
                        && let Some(code) = Scancode::from_name(value)
                    {
                        settings.keys[i] = code;
                    }
                }
            }
        }
        settings
    }

    pub fn save(&self) {
        // the tests change settings: never the player's file
        if cfg!(test) {
            return;
        }
        let Some(path) = path() else { return };
        let on = |b: bool| if b { "on" } else { "off" };
        let mut text = format!("music = {}\nsounds = {}\n", on(self.music), on(self.sounds));
        text += &format!(
            "screen = {}\n",
            if self.fullscreen { "full" } else { "window" }
        );
        text += &format!("scaling = {}\n", self.scaling.word());
        for (name, key) in KEY_NAMES.iter().zip(self.keys) {
            text += &format!("{} = {}\n", name.to_lowercase(), key.name());
        }
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("cannot save the settings to {}: {e}", path.display());
        }
    }

    /// Give `action` the key `code`; an action that had it gets the old key
    /// of `action`, so no key does two things. (The original warns about
    /// identical keys instead, and forgets to check the fire key.)
    pub fn set_key(&mut self, action: usize, code: Scancode) {
        if let Some(other) = self.keys.iter().position(|&k| k == code) {
            self.keys[other] = self.keys[action];
        }
        self.keys[action] = code;
    }
}

/// Keys a game action cannot have: the shortcuts' modifiers (Command /
/// Ctrl), full screen (F11) and the timeline (Backspace).
pub fn reserved(code: Scancode) -> bool {
    matches!(code, Scancode::LGui | Scancode::RGui | Scancode::LCtrl | Scancode::RCtrl | Scancode::F11 | Scancode::Backspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_in_use_is_swapped() {
        let mut s = Settings::default();
        s.set_key(0, Scancode::Left);
        assert_eq!(s.keys[0], Scancode::Left);
        assert_eq!(s.keys[1], Scancode::Right);
    }
}
