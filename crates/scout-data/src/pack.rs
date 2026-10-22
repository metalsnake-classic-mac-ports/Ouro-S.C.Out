//! The data pack: a zip file of ordinary files (PNG, WAV, FLAC, text), made
//! by `ouro-scout-extract` (crate `scout-original`) from the original and read by
//! the game. It holds no original resource data, only what the game uses,
//! converted:
//!
//! - `pack.txt`: [`FORMAT`] on the first line
//! - `palette.gpl`: the colors of the game screen (GIMP palette, 256)
//! - `graphics/tiles/NNN.png` (480), `objects/NN.png` (39, with
//!   transparency), `font/NN.png` and `small_font/NN.png` (45 block
//!   letters), `box/N.png` (9 marble tiles), `icons/NN.png` (24),
//!   `frame/{top,bottom,left,right}.png`; their colors are mapped to the
//!   game palette when read (the nearest color for a new one)
//! - `pictures/ID.png`: the pictures, each with its own colors (at most 256)
//! - `sounds/ID.wav`: 8-bit mono (16-bit is converted)
//! - `music/music.flac`: the music once through, played in a loop (mono,
//!   16 bits)
//! - `levels/NNN.txt`: the values of the level file (24 rows of 40 tiles,
//!   then 24 rows of 40 floor values; lines with `#` are comments),
//!   `levels/codes.txt`: the level codes, one per line (level 2 first)

use std::collections::HashMap;
use std::io::{Cursor, Read};

use scout_core::Level;

use crate::error::{Result, corrupt};
use crate::graphics::{Bitmap, Frame, IndexedImage, ObjectSet, TileSet};
use crate::palette::{Palette, Rgb};
use crate::sound::{Music, Sound};
use crate::{GameData, NamedPalette, Picture};

/// The first line of `pack.txt`.
pub const FORMAT: &str = "S.C.Out data pack 1";
pub const TILE: usize = 32;

/// Whether these bytes look like a zip file.
pub fn is_pack(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
}

impl GameData {
    /// Read a pack (the bytes of the zip file).
    pub fn from_pack(bytes: &[u8]) -> Result<Self> {
        let mut zip =
            zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| corrupt(format!("pack: {e}")))?;
        let mut read = |name: &str| -> Result<Vec<u8>> {
            let mut file = zip
                .by_name(name)
                .map_err(|_| corrupt(format!("pack: {name} missing")))?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|e| corrupt(format!("pack: {name}: {e}")))?;
            Ok(data)
        };
        let text = |data: Vec<u8>| String::from_utf8_lossy(&data).into_owned();
        if !text(read("pack.txt")?).starts_with(FORMAT) {
            return Err(corrupt(format!("pack.txt does not start with {FORMAT:?}")));
        }
        let game_palette = parse_gpl(&text(read("palette.gpl")?))?;
        let colors = ColorMap::new(&game_palette);

        let mut tiles = Vec::new();
        for i in 0.. {
            let Ok(png) = read(&format!("graphics/tiles/{i:03}.png")) else {
                break;
            };
            tiles.extend(colors.indices(&decode_png(&png, TILE, TILE)?));
        }
        let objects = object_set(&mut read, &colors, "objects", TILE)?;
        let font = object_set(&mut read, &colors, "font", TILE)?;
        let small_font = object_set(&mut read, &colors, "small_font", 16)?;
        let mut box_tiles = Vec::new();
        for i in 0..9 {
            box_tiles.extend(colors.indices(&decode_png(
                &read(&format!("graphics/box/{i}.png"))?,
                TILE,
                TILE,
            )?));
        }
        let mut icons = Vec::new();
        for i in 0.. {
            let Ok(png) = read(&format!("graphics/icons/{i:02}.png")) else {
                break;
            };
            let img = decode_png_any(&png)?;
            icons.push(Bitmap::new(img.width, img.height, colors.indices(&img))?);
        }
        let mut bitmap = |name: &str, w: usize, h: usize| -> Result<Bitmap> {
            let img = decode_png(&read(&format!("graphics/frame/{name}.png"))?, w, h)?;
            Bitmap::new(w, h, colors.indices(&img))
        };
        let frame = Frame {
            top: bitmap("top", 448, 32)?,
            bottom: bitmap("bottom", 448, 12)?,
            left: bitmap("left", 32, Frame::HEIGHT)?,
            right: bitmap("right", 32, Frame::HEIGHT)?,
        };

        let mut pictures = Vec::new();
        let mut palettes = vec![NamedPalette {
            name: "game".into(),
            palette: game_palette.clone(),
        }];
        let names: Vec<String> = zip_names(&mut zip);
        let mut picture_ids: Vec<i16> = names
            .iter()
            .filter_map(|n| {
                n.strip_prefix("pictures/")?
                    .strip_suffix(".png")?
                    .parse()
                    .ok()
            })
            .collect();
        picture_ids.sort();
        let mut read = |name: &str| -> Result<Vec<u8>> {
            let mut file = zip
                .by_name(name)
                .map_err(|_| corrupt(format!("pack: {name} missing")))?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|e| corrupt(format!("pack: {name}: {e}")))?;
            Ok(data)
        };
        for id in picture_ids {
            let image = picture(&decode_png_any(&read(&format!("pictures/{id}.png"))?)?)?;
            palettes.push(NamedPalette {
                name: format!("PICT {id}"),
                palette: image.palette.clone(),
            });
            pictures.push(Picture {
                id,
                name: String::new(),
                image,
            });
        }
        let mut sounds = Vec::new();
        for id in crate::SOUND_IDS {
            sounds.push(parse_wav(id, &read(&format!("sounds/{id}.wav"))?)?);
        }
        let music = parse_flac(&read("music/music.flac")?)?;
        let mut levels = Vec::new();
        for i in 1.. {
            let Ok(data) = read(&format!("levels/{i:03}.txt")) else {
                break;
            };
            levels.push(
                parse_level(&text(data)).map_err(|e| corrupt(format!("levels/{i:03}.txt: {e}")))?,
            );
        }
        let level_codes = crate::parse_codes(&read("levels/codes.txt")?);

        Ok(Self {
            levels,
            tiles: TileSet::new(tiles)?,
            objects,
            frame,
            icons,
            sounds,
            music,
            font,
            small_font,
            box_tiles: TileSet::new(box_tiles)?,
            game_palette,
            level_codes,
            palettes,
            pictures,
        })
    }
}

fn zip_names(zip: &mut zip::ZipArchive<Cursor<&[u8]>>) -> Vec<String> {
    zip.file_names()
        .filter_map(|n| n.ok().map(|n| n.into_owned()))
        .collect()
}

fn object_set(
    read: &mut impl FnMut(&str) -> Result<Vec<u8>>,
    colors: &ColorMap,
    dir: &str,
    size: usize,
) -> Result<ObjectSet> {
    let mut items = Vec::new();
    for i in 0.. {
        let Ok(png) = read(&format!("graphics/{dir}/{i:02}.png")) else {
            break;
        };
        let img = decode_png(&png, size, size)?;
        let indices = colors.indices(&img);
        items.push(
            indices
                .iter()
                .zip(img.rgba.as_chunks::<4>().0)
                .map(|(&i, px)| (px[3] >= 128).then_some(i))
                .collect(),
        );
    }
    ObjectSet::from_pixels(size, &items)
}

/// An RGBA image.
struct Rgba {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

/// Colors to the indices of a palette: the first index of an exact color,
/// else the nearest.
struct ColorMap<'a> {
    palette: &'a Palette,
    exact: HashMap<(u8, u8, u8), u8>,
}

impl<'a> ColorMap<'a> {
    fn new(palette: &'a Palette) -> Self {
        let mut exact = HashMap::new();
        for (i, c) in palette.colors.iter().enumerate() {
            exact.entry((c.r, c.g, c.b)).or_insert(i as u8);
        }
        Self { palette, exact }
    }

    fn index(&self, r: u8, g: u8, b: u8) -> u8 {
        if let Some(&i) = self.exact.get(&(r, g, b)) {
            return i;
        }
        let distance = |c: &Rgb| {
            let d = |a: u8, b: u8| (a as i32 - b as i32).pow(2);
            d(c.r, r) + d(c.g, g) + d(c.b, b)
        };
        (0..256)
            .min_by_key(|&i| distance(&self.palette.colors[i]))
            .unwrap() as u8
    }

    fn indices(&self, img: &Rgba) -> Vec<u8> {
        img.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| self.index(p[0], p[1], p[2]))
            .collect()
    }
}

/// A picture with its own colors: the colors in the order they appear.
fn picture(img: &Rgba) -> Result<IndexedImage> {
    let mut palette = Palette {
        colors: [Rgb::default(); 256],
    };
    let mut seen: HashMap<(u8, u8, u8), u8> = HashMap::new();
    let mut pixels = Vec::with_capacity(img.width * img.height);
    for p in img.rgba.as_chunks::<4>().0 {
        let key = (p[0], p[1], p[2]);
        let n = seen.len();
        let i = match seen.get(&key) {
            Some(&i) => i,
            None if n < 256 => {
                palette.colors[n] = Rgb {
                    r: p[0],
                    g: p[1],
                    b: p[2],
                };
                seen.insert(key, n as u8);
                n as u8
            }
            None => return Err(corrupt("a picture has more than 256 colors")),
        };
        pixels.push(i);
    }
    Ok(IndexedImage {
        width: img.width,
        height: img.height,
        pixels,
        palette,
    })
}

fn decode_png_any(data: &[u8]) -> Result<Rgba> {
    let mut decoder = png::Decoder::new(Cursor::new(data));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| corrupt(format!("PNG: {e}")))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| corrupt(format!("PNG: {e}")))?;
    let (width, height) = (info.width as usize, info.height as usize);
    let channels = info.color_type.samples();
    let mut rgba = Vec::with_capacity(width * height * 4);
    for px in buf[..info.buffer_size()].chunks_exact(channels) {
        rgba.extend(match channels {
            1 => [px[0], px[0], px[0], 255],
            2 => [px[0], px[0], px[0], px[1]],
            3 => [px[0], px[1], px[2], 255],
            _ => [px[0], px[1], px[2], px[3]],
        });
    }
    Ok(Rgba {
        width,
        height,
        rgba,
    })
}

fn decode_png(data: &[u8], width: usize, height: usize) -> Result<Rgba> {
    let img = decode_png_any(data)?;
    if (img.width, img.height) != (width, height) {
        return Err(corrupt(format!(
            "an image is {} × {}, not {width} × {height}",
            img.width, img.height
        )));
    }
    Ok(img)
}

fn parse_gpl(text: &str) -> Result<Palette> {
    let mut palette = Palette {
        colors: [Rgb::default(); 256],
    };
    let mut n = 0;
    for line in text.lines().skip(1) {
        let mut parts = line.split_whitespace().map(str::parse::<u8>);
        if let (Some(Ok(r)), Some(Ok(g)), Some(Ok(b))) = (parts.next(), parts.next(), parts.next())
        {
            if n < 256 {
                palette.colors[n] = Rgb { r, g, b };
            }
            n += 1;
        }
    }
    if n != 256 {
        return Err(corrupt(format!("palette.gpl has {n} colors, not 256")));
    }
    Ok(palette)
}

/// A PCM WAV, mono or stereo (the left channel), 8 or 16 bits, as 8-bit
/// unsigned mono.
fn parse_wav(id: i16, data: &[u8]) -> Result<Sound> {
    let bad = || corrupt(format!("sounds/{id}.wav is not a PCM WAV"));
    if data.get(0..4) != Some(b"RIFF") || data.get(8..12) != Some(b"WAVE") {
        return Err(bad());
    }
    let (mut format, mut samples) = (None, None);
    let mut at = 12;
    while at + 8 <= data.len() {
        let len = u32::from_le_bytes(data[at + 4..at + 8].try_into().unwrap()) as usize;
        let body = data.get(at + 8..at + 8 + len).ok_or_else(bad)?;
        match &data[at..at + 4] {
            b"fmt " if len >= 16 => {
                let u16_at = |i: usize| u16::from_le_bytes([body[i], body[i + 1]]);
                let rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
                format = Some((u16_at(0), u16_at(2) as usize, rate, u16_at(14)));
            }
            b"data" => samples = Some(body),
            _ => {}
        }
        at += 8 + len + (len & 1);
    }
    let ((1, channels @ 1..=2, rate, bits @ (8 | 16)), Some(body)) =
        (format.ok_or_else(bad)?, samples)
    else {
        return Err(bad());
    };
    let frame = channels * bits as usize / 8;
    let samples = body
        .chunks_exact(frame)
        .map(|f| match bits {
            8 => f[0],
            _ => (i16::from_le_bytes([f[0], f[1]]) >> 8) as u8 ^ 0x80,
        })
        .collect();
    Ok(Sound { id, rate, samples })
}

fn numbers<T: std::str::FromStr>(text: &str) -> Result<Vec<T>> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .flat_map(str::split_whitespace)
        .map(|n| {
            n.parse()
                .map_err(|_| corrupt(format!("{n:?} is not a number")))
        })
        .collect()
}

fn parse_level(text: &str) -> Result<Level> {
    let values: Vec<u16> = numbers(text)?;
    if values.len() != 2 * Level::CELLS {
        return Err(corrupt(format!(
            "{} values, not {}",
            values.len(),
            2 * Level::CELLS
        )));
    }
    let tiles: [u16; Level::CELLS] = values[..Level::CELLS].try_into().unwrap();
    let mut floor = [0u8; Level::CELLS];
    for (f, &v) in floor.iter_mut().zip(&values[Level::CELLS..]) {
        *f = u8::try_from(v).map_err(|_| corrupt(format!("floor value {v}")))?;
    }
    Ok(Level::from_file(tiles, floor))
}

/// A FLAC file as 16-bit mono (the first channel).
fn parse_flac(data: &[u8]) -> Result<Music> {
    let bad = |e: claxon::Error| corrupt(format!("music/music.flac: {e}"));
    let mut reader = claxon::FlacReader::new(Cursor::new(data)).map_err(bad)?;
    let info = reader.streaminfo();
    let (channels, bits) = (info.channels as usize, info.bits_per_sample);
    let samples: Vec<i32> = reader
        .samples()
        .collect::<std::result::Result<_, _>>()
        .map_err(bad)?;
    let samples = samples
        .chunks(channels)
        .map(|frame| {
            let s = frame[0];
            (if bits >= 16 {
                s >> (bits - 16)
            } else {
                s << (16 - bits)
            }) as i16
        })
        .collect();
    Ok(Music {
        rate: info.sample_rate,
        samples,
    })
}
