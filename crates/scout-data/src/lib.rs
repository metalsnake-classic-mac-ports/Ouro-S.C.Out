//! Reads the game data from the original S.C.OUT Mac disk image
//! (`SCOUT_1_4.dsk`, HFS floppy, version 1.4 from 1993).
//!
//! Everything lives in the resource fork of the application "S.C.OUT".
//! Levels, graphics, codes and music are 'SHAP' resources compressed with
//! LZRW1-A; palettes are 'pltt' resources, pictures are 'PICT' resources.

mod bytes;
pub mod error;
pub mod graphics;
pub mod hfs;
pub mod lzrw1a;
pub mod palette;
pub mod pict;
pub mod resource;

use std::path::Path;

use scout_core::Level;

pub use error::{Error, Result};
pub use graphics::{Bitmap, Frame, ObjectSet, TileSet};
pub use palette::{Palette, Rgb};
pub use pict::IndexedImage;
use resource::ResourceFork;

const APPLICATION: &str = "S.C.OUT";
const SHAP: &[u8; 4] = b"SHAP";
const OBJECTS_ID: i16 = 5100;
const TILES_ID: i16 = 5101;
const FRAME_TOP_ID: i16 = 5102;
const FRAME_BOTTOM_ID: i16 = 5103;
const FRAME_SIDES_ID: i16 = 5104;
const ICONS_ID: i16 = 5105;
const FONT_ID: i16 = 5106;
const BOX_ID: i16 = 5108;
const CODES_ID: i16 = 5300;
const LEVELS_ID: i16 = 6642;
const GAME_PALETTE_ID: i16 = 128;
const LEVEL_BYTES: usize = 2880;
const ICON_WIDTH: usize = 28;
const ICON_HEIGHT: usize = 19;

/// All game data needed by the game, loaded from the original disk.
#[derive(Debug, Clone)]
pub struct GameData {
    pub levels: Vec<Level>,
    pub tiles: TileSet,
    pub objects: ObjectSet,
    pub frame: Frame,
    /// The carried object in the side panels (CODE 9 0x5416): 28 × 19
    /// pixels, icon = object − 16, 23 = nothing.
    pub icons: Vec<Bitmap>,
    /// The block letters of the screens between levels (CODE 8 0x19c2):
    /// A–Z, 1–9, 0, then `.` at 38, `:` at 40, `-` at 44.
    pub font: ObjectSet,
    /// The 3 × 3 tiles of the boxes these screens are drawn in (CODE 8
    /// 0x15ce): corners, edges and the inside, row by row.
    pub box_tiles: TileSet,
    /// The colors of the game screen: palette index → color for the tiles
    /// and objects (see [`Palette::from_palette_manager`]).
    pub game_palette: Palette,
    /// Level codes; `level_codes[0]` is the code of level 2.
    pub level_codes: Vec<String>,
    /// All palettes: the 'pltt' resources and the color tables of the pictures.
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
    /// Load from the disk image file `SCOUT_1_4.dsk`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let image = std::fs::read(path).map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_disk_image(&image)
    }

    pub fn from_disk_image(image: &[u8]) -> Result<Self> {
        let volume = hfs::Volume::open(image)?;
        let app = volume.file(APPLICATION)?;
        Self::from_resource_fork(&volume.resource_fork(&app)?)
    }

    pub fn from_resource_fork(fork: &[u8]) -> Result<Self> {
        let res = ResourceFork::parse(fork)?;
        let shap = |id| -> Result<Vec<u8>> { lzrw1a::unpack_shap(&res.get(SHAP, id)?.data) };

        let mut palettes = Vec::new();
        let game_palette = Palette::from_palette_manager(&Palette::from_pltt(
            &res.get(b"pltt", GAME_PALETTE_ID)?.data,
        )?);
        palettes.push(NamedPalette {
            name: "game".to_string(),
            palette: game_palette.clone(),
        });
        let mut pltts: Vec<_> = res.of_kind(b"pltt").collect();
        pltts.sort_by_key(|r| r.id);
        for r in pltts {
            palettes.push(NamedPalette {
                name: format!("pltt {}", r.id),
                palette: Palette::from_pltt(&r.data)?,
            });
        }
        let mut pictures = Vec::new();
        let mut picts: Vec<_> = res.of_kind(b"PICT").collect();
        picts.sort_by_key(|r| r.id);
        for r in picts {
            let image = pict::decode(&r.data)?;
            palettes.push(NamedPalette {
                name: format!("PICT {}", r.id),
                palette: image.palette.clone(),
            });
            pictures.push(Picture {
                id: r.id,
                name: r.name.clone(),
                image,
            });
        }

        Ok(Self {
            levels: parse_levels(&shap(LEVELS_ID)?)?,
            tiles: TileSet::new(shap(TILES_ID)?)?,
            objects: ObjectSet::new(shap(OBJECTS_ID)?)?,
            frame: Frame::new(
                shap(FRAME_TOP_ID)?,
                shap(FRAME_BOTTOM_ID)?,
                shap(FRAME_SIDES_ID)?,
            )?,
            icons: shap(ICONS_ID)?
                .as_chunks::<{ ICON_WIDTH * ICON_HEIGHT }>()
                .0
                .iter()
                .map(|c| Bitmap::new(ICON_WIDTH, ICON_HEIGHT, c.to_vec()))
                .collect::<Result<_>>()?,
            font: ObjectSet::new(shap(FONT_ID)?)?,
            box_tiles: TileSet::new(shap(BOX_ID)?)?,
            game_palette,
            level_codes: parse_codes(&shap(CODES_ID)?),
            palettes,
            pictures,
        })
    }
}

/// Levels: 960 tile values (16 bit little-endian), then 960 floor bytes.
fn parse_levels(data: &[u8]) -> Result<Vec<Level>> {
    if !data.len().is_multiple_of(LEVEL_BYTES) {
        return Err(Error::Corrupt(format!(
            "level data is {} bytes, not a multiple of {LEVEL_BYTES}",
            data.len()
        )));
    }
    Ok(data
        .as_chunks::<LEVEL_BYTES>()
        .0
        .iter()
        .map(|chunk| {
            let mut tiles = [0u16; Level::CELLS];
            for (i, tile) in tiles.iter_mut().enumerate() {
                *tile = u16::from_le_bytes([chunk[2 * i], chunk[2 * i + 1]]);
            }
            let floor: [u8; Level::CELLS] = chunk[2 * Level::CELLS..].try_into().unwrap();
            Level::from_file(tiles, floor)
        })
        .collect())
}

fn parse_codes(data: &[u8]) -> Vec<String> {
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
    fn levels_are_little_endian_tiles_then_floor() {
        let mut data = vec![0u8; LEVEL_BYTES];
        data[2..4].copy_from_slice(&300u16.to_le_bytes()); // box (1, 0)
        data[2 * Level::CELLS + Level::WIDTH] = 41; // floor of box (0, 1)
        let levels = parse_levels(&data).unwrap();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].file_tile(1, 0), 300);
        assert_eq!(levels[0].file_floor(0, 1), 41);
        assert!(parse_levels(&data[1..]).is_err());
    }

    #[test]
    fn codes_are_lines() {
        assert_eq!(
            parse_codes(b"RAFAFE\r\nLAXUDI\r\n\r\n"),
            ["RAFAFE", "LAXUDI"]
        );
    }
}
