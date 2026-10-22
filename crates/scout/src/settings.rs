//! The settings (Game → Setting in the original, CODE 2 0x1108): music,
//! sound effects and the six game keys, plus our own options. The original
//! keeps them in its own resource `SHAP 1` (`SHAP 0` if it is missing) and
//! writes that back into the application when it quits (CODE 5 0x16ce); we
//! keep them in a text file in the user's settings folder.

use std::path::PathBuf;

use sdl3::keyboard::Scancode;

/// The game keys in the order of the original dialog, then self destruct
/// (fixed to Esc in the original).
pub const KEY_NAMES: [&str; 7] = [
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
    pub keys: [Scancode; 7],
    pub fullscreen: bool,
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
        }
    }
}

const FILE: &str = "settings.txt";

/// The settings folder (SDL's preference folder, macOS:
/// `~/Library/Application Support/Ouro S.C.Out`).
const FOLDER: &str = "Ouro S.C.Out";
/// Its name before 2026-10-22; settings found there are taken over.
const OLD_FOLDER: &str = "S.C.Out";

fn path() -> Option<PathBuf> {
    let path = sdl3::filesystem::get_pref_path("", FOLDER).ok()?.join(FILE);
    if !path.exists()
        && let Some(old) = sdl3::filesystem::get_pref_path("", OLD_FOLDER)
            .ok()
            .map(|d| d.join(FILE))
            .filter(|p| p.exists())
    {
        let _ = std::fs::copy(old, &path);
    }
    Some(path)
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
        let Some(path) = path() else { return };
        let on = |b: bool| if b { "on" } else { "off" };
        let mut text = format!("music = {}\nsounds = {}\n", on(self.music), on(self.sounds));
        text += &format!(
            "screen = {}\n",
            if self.fullscreen { "full" } else { "window" }
        );
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
