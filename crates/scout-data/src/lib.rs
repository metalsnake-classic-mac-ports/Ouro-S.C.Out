//! The game data of S.C.Out and its data pack (see [`pack`]): what the game
//! reads. `scout-original` makes the pack from the original Mac data.

pub mod error;
pub mod graphics;
pub mod pack;
pub mod palette;
pub mod sound;

use std::path::Path;

use scout_core::Level;

pub use error::{Error, Result};
pub use graphics::{Bitmap, Frame, IndexedImage, ObjectSet, TileSet};
pub use palette::{Palette, Rgb};
pub use sound::{Music, Sound};

/// The numbers of the sound effects (as the 'snd ' resources 100–124 of the
/// original, see re/NOTES.md, Sound).
pub const SOUND_IDS: std::ops::RangeInclusive<i16> = 100..=124;

/// All data the game needs.
#[derive(Debug, Clone)]
pub struct GameData {
    pub levels: Vec<Level>,
    pub tiles: TileSet,
    pub objects: ObjectSet,
    pub frame: Frame,
    /// The carried object in the side panels (CODE 9 0x5416): 28 × 19
    /// pixels, icon = object − 16, 23 = nothing.
    pub icons: Vec<Bitmap>,
    /// Sound effects by number (100–124).
    pub sounds: Vec<Sound>,
    /// The music, once through from the start.
    pub music: Music,
    /// The block letters of the screens between levels (CODE 8 0x19c2):
    /// A–Z, 1–9, 0, then `! ? . , : ; = + -` (36–44).
    pub font: ObjectSet,
    /// The same letters at 16 × 16 (`SHAP 5107`); the game does not use them.
    pub small_font: ObjectSet,
    /// The 3 × 3 tiles of the boxes these screens are drawn in (CODE 8
    /// 0x15ce): corners, edges and the inside, row by row.
    pub box_tiles: TileSet,
    /// The colors of the game screen: palette index → color for the tiles,
    /// objects and everything else drawn in the game's colors.
    pub game_palette: Palette,
    /// Level codes; `level_codes[0]` is the code of level 2.
    pub level_codes: Vec<String>,
    /// The game palette ("game"), the pictures' palettes ("PICT id") and,
    /// from the original, its other palettes ("pltt id").
    pub palettes: Vec<NamedPalette>,
    pub pictures: Vec<Picture>,
}

#[derive(Debug, Clone)]
pub struct NamedPalette {
    pub name: String,
    pub palette: Palette,
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub id: i16,
    pub name: String,
    pub image: IndexedImage,
}

impl GameData {
    /// Load a data pack (`ouro-scout-data.zip`).
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })?;
        if !pack::is_pack(&bytes) {
            return Err(error::corrupt(format!(
                "{} is not a data pack (make one with ouro-scout-extract)",
                path.display()
            )));
        }
        Self::from_pack(&bytes)
    }
}

/// Level codes: one per line.
pub fn parse_codes(data: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(data)
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_lines() {
        assert_eq!(
            parse_codes(b"RAFAFE\r\nLAXUDI\r\n\r\n"),
            ["RAFAFE", "LAXUDI"]
        );
    }
}
