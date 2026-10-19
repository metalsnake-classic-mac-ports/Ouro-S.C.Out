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
pub mod translate;

use std::path::Path;

use scout_core::Level;

pub use error::{Error, Result};
pub use graphics::{ObjectSet, TileSet};
pub use palette::{Palette, Rgb};
pub use pict::IndexedImage;
use resource::ResourceFork;

const APPLICATION: &str = "S.C.OUT";
const SHAP: &[u8; 4] = b"SHAP";
const OBJECTS_ID: i16 = 5100;
const TILES_ID: i16 = 5101;
const CODES_ID: i16 = 5300;
const LEVELS_ID: i16 = 6642;
const LEVEL_BYTES: usize = 2880;

/// All game data needed by the game, loaded from the original disk.
#[derive(Debug, Clone)]
pub struct GameData {
    pub levels: Vec<Level>,
    pub tiles: TileSet,
    pub objects: ObjectSet,
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
            Level::new(tiles, floor)
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
        assert_eq!(levels[0].tile(1, 0), 300);
        assert_eq!(levels[0].floor(0, 1), 41);
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
