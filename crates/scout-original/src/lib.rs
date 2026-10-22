//! Reads the original S.C.Out data (Mac, Kalisto 1993): the disk image
//! `SCOUT_1_4.dsk` (HFS floppy) or the application itself. Everything lives
//! in the resource fork of the application "S.C.OUT": levels, graphics,
//! codes and music are 'SHAP' resources compressed with LZRW1-A, palettes
//! 'pltt', pictures 'PICT', sounds 'snd ', the music player CODE 10.
//!
//! The game itself reads the data pack (`scout_data::pack`) that
//! [`pack::write`] makes from this (tool `ouro-scout-extract`).

mod bytes;
pub mod error;
pub mod hfs;
pub mod lzrw1a;
pub mod music;
pub mod pack;
pub mod palette;
pub mod pict;
pub mod resource;
pub mod snd;

use std::path::Path;

use scout_core::Level;
use scout_data::{
    Bitmap, Frame, GameData, Music, NamedPalette, ObjectSet, Picture, SOUND_IDS, TileSet,
};

pub use error::{Error, Result};
use resource::ResourceFork;

/// The game on the disk: an application by Kalisto (creator 'KIRI'),
/// normally named "S.C.OUT"; renamed copies (e.g. "S.C.OUT 1.3.1") work too.
const APPLICATION: &str = "S.C.OUT";
const APPLICATION_TYPE: &[u8; 4] = b"APPL";
const CREATOR: &[u8; 4] = b"KIRI";
const SHAP: &[u8; 4] = b"SHAP";
const OBJECTS_ID: i16 = 5100;
const TILES_ID: i16 = 5101;
const FRAME_TOP_ID: i16 = 5102;
const FRAME_BOTTOM_ID: i16 = 5103;
const FRAME_SIDES_ID: i16 = 5104;
const ICONS_ID: i16 = 5105;
/// The music, an uncompressed ProTracker module (CODE 5 0x1f6).
const MUSIC_ID: i16 = 200;
/// The music player (its volume table is read from the code).
const MUSIC_CODE_ID: i16 = 10;
/// An earlier version of the music ("2ul"), packed; the game never loads it.
const UNUSED_MUSIC_ID: i16 = 5200;
const FONT_ID: i16 = 5106;
const SMALL_FONT_ID: i16 = 5107;
const BOX_ID: i16 = 5108;
const CODES_ID: i16 = 5300;
const LEVELS_ID: i16 = 6642;
const GAME_PALETTE_ID: i16 = 128;
const LEVEL_BYTES: usize = 2880;
const ICON_WIDTH: usize = 28;
const ICON_HEIGHT: usize = 19;
/// At most this long a song is rendered (the music takes about 3 minutes).
const MAX_SONG_SECONDS: f64 = 600.0;

/// How the music is rendered: by the original player with its bugs, or with
/// them fixed (see re/BUGS.md).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MusicVersion {
    Original,
    #[default]
    Fixed,
}

/// The original data: the game data (with the music rendered) and what only
/// the tools need.
#[derive(Debug, Clone)]
pub struct Original {
    pub game: GameData,
    /// The music: 'SHAP' 200, a ProTracker module.
    pub module: Vec<u8>,
    /// 'SHAP' 5200 unpacked: an earlier version of the music, unused.
    pub unused_module: Vec<u8>,
    /// The gains of the music player's volume table (from CODE 10).
    pub volumes: music::Volumes,
}

/// Load the disk image `SCOUT_1_4.dsk`, or the application itself: its
/// resource fork (on macOS `FILE/..namedfork/rsrc`) or a file holding the
/// resource fork.
pub fn load(path: impl AsRef<Path>, music: MusicVersion) -> Result<Original> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })?;
    match from_disk_image(&bytes, music) {
        Err(Error::NotHfs) => {
            let fork = std::fs::read(path.join("..namedfork/rsrc"))
                .ok()
                .filter(|f| !f.is_empty())
                .unwrap_or(bytes);
            from_resource_fork(&fork, music)
        }
        other => other,
    }
}

pub fn from_disk_image(image: &[u8], music: MusicVersion) -> Result<Original> {
    let volume = hfs::Volume::open(image)?;
    let files = volume.files()?;
    let mut apps: Vec<_> = files
        .iter()
        .filter(|f| &f.file_type == APPLICATION_TYPE && &f.creator == CREATOR)
        .collect();
    apps.sort_by_key(|f| !f.name.eq_ignore_ascii_case(APPLICATION));
    let app = match apps.first() {
        Some(app) => (*app).clone(),
        None => volume.file(APPLICATION)?,
    };
    from_resource_fork(&volume.resource_fork(&app)?, music)
}

pub fn from_resource_fork(fork: &[u8], music: MusicVersion) -> Result<Original> {
    let res = ResourceFork::parse(fork)?;
    let shap = |id| -> Result<Vec<u8>> { lzrw1a::unpack_shap(&res.get(SHAP, id)?.data) };

    let mut palettes = Vec::new();
    let game_palette = palette::from_palette_manager(&palette::from_pltt(
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
            palette: palette::from_pltt(&r.data)?,
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
    let module = res.get(SHAP, MUSIC_ID)?.data.clone();
    let volumes = res
        .get(b"CODE", MUSIC_CODE_ID)?
        .data
        .get(4..)
        .and_then(music::volumes_from_code)
        .ok_or_else(|| error::corrupt("no volume table in CODE 10"))?;

    let game = GameData {
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
            .map(|c| Ok(Bitmap::new(ICON_WIDTH, ICON_HEIGHT, c.to_vec())?))
            .collect::<Result<_>>()?,
        sounds: SOUND_IDS
            .map(|id| snd::sound(id, &res.get(b"snd ", id)?.data))
            .collect::<Result<_>>()?,
        music: render_music(&module, &volumes, music),
        font: ObjectSet::new(shap(FONT_ID)?)?,
        small_font: ObjectSet::with_size(shap(SMALL_FONT_ID)?, 16)?,
        box_tiles: TileSet::new(shap(BOX_ID)?)?,
        game_palette,
        level_codes: scout_data::parse_codes(&shap(CODES_ID)?),
        palettes,
        pictures,
    };
    Ok(Original {
        game,
        module,
        unused_module: shap(UNUSED_MUSIC_ID)?,
        volumes,
    })
}

/// The music once through from position 0, as the player plays it, in 16
/// bits.
pub fn render_music(module: &[u8], volumes: &music::Volumes, version: MusicVersion) -> Music {
    let max_ticks = (MAX_SONG_SECONDS * music::RATE) as usize / music::TICK_SAMPLES;
    let song =
        music::MusicPlayer::render_song(module, volumes, version == MusicVersion::Fixed, max_ticks);
    Music {
        rate: music::RATE.round() as u32,
        samples: song.iter().map(|&s| ((s as i16) - 128) << 8).collect(),
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
}
