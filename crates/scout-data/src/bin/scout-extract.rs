//! Export everything the game reads from the original disk into ordinary
//! files, for tests and for looking at:
//!
//! - `graphics/`: tiles, objects, block letters, box tiles and side panel
//!   icons (one PNG each plus a sheet), the frame around the view and the
//!   pictures, in the colors of the game (objects and letters with
//!   transparency where their mask is clear)
//! - `sounds/`: the sound effects as WAV (8-bit mono, original rate)
//! - `levels/`: each level as text, the translated tile and floor values
//!   the game uses and the values of the level file
//! - `codes.txt`, `palette.txt`
//!
//! Usage: scout-extract [DISK] OUT_DIR
//! DISK defaults to `re/work/SCOUT_1_4.dsk`.

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use scout_core::Level;
use scout_data::graphics::{ObjectSet, TileSet};
use scout_data::{Bitmap, GameData, Palette, Sound};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const TILE: usize = 32;
const PER_ROW: usize = 16;
const GAP: usize = 2;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (disk, out) = match args.as_slice() {
        [out] => (PathBuf::from("re/work/SCOUT_1_4.dsk"), PathBuf::from(out)),
        [disk, out] => (PathBuf::from(disk), PathBuf::from(out)),
        _ => return Err("usage: scout-extract [DISK] OUT_DIR".into()),
    };
    let data = GameData::load(&disk)?;
    let pal = &data.game_palette;

    let gfx = out.join("graphics");
    tiles(&gfx.join("tiles"), &data.tiles, pal)?;
    tiles(&gfx.join("box"), &data.box_tiles, pal)?;
    objects(&gfx.join("objects"), &data.objects, pal)?;
    objects(&gfx.join("font"), &data.font, pal)?;
    let icons = gfx.join("icons");
    fs::create_dir_all(&icons)?;
    for (i, icon) in data.icons.iter().enumerate() {
        bitmap(&icons.join(format!("{i:02}.png")), icon, pal)?;
    }
    let frame = gfx.join("frame");
    fs::create_dir_all(&frame)?;
    for (name, part) in [
        ("top", &data.frame.top),
        ("bottom", &data.frame.bottom),
        ("left", &data.frame.left),
        ("right", &data.frame.right),
    ] {
        bitmap(&frame.join(format!("{name}.png")), part, pal)?;
    }
    let pictures = gfx.join("pictures");
    fs::create_dir_all(&pictures)?;
    for p in &data.pictures {
        let rgba = indexed(&p.image.pixels, &p.image.palette, None);
        png(
            &pictures.join(format!("{}_{}.png", p.id, safe(&p.name))),
            p.image.width,
            p.image.height,
            &rgba,
        )?;
    }

    let sounds = out.join("sounds");
    fs::create_dir_all(&sounds)?;
    for s in &data.sounds {
        wav(&sounds.join(format!("snd_{}.wav", s.id)), s)?;
    }

    let levels = out.join("levels");
    fs::create_dir_all(&levels)?;
    for (i, level) in data.levels.iter().enumerate() {
        let code = i.checked_sub(1).map(|c| data.level_codes[c].as_str());
        fs::write(
            levels.join(format!("level_{:03}.txt", i + 1)),
            level_text(i + 1, code, level),
        )?;
    }
    let codes: String = data
        .level_codes
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{} {c}\n", i + 2))
        .collect();
    fs::write(out.join("codes.txt"), codes)?;
    let palette: String = pal
        .colors
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{i} {} {} {}\n", c.r, c.g, c.b))
        .collect();
    fs::write(out.join("palette.txt"), palette)?;
    println!(
        "{}: {} tiles, {} objects, {} letters, {} icons, {} pictures, {} sounds, {} levels",
        out.display(),
        data.tiles.len(),
        data.objects.len(),
        data.font.len(),
        data.icons.len(),
        data.pictures.len(),
        data.sounds.len(),
        data.levels.len()
    );
    Ok(())
}

fn safe(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// RGBA pixels from palette indices; `mask` false = transparent.
fn indexed(pixels: &[u8], pal: &Palette, mask: Option<&[bool]>) -> Vec<u8> {
    pixels
        .iter()
        .enumerate()
        .flat_map(|(i, &p)| {
            let c = pal.colors[p as usize];
            let alpha = if mask.is_none_or(|m| m[i]) { 255 } else { 0 };
            [c.r, c.g, c.b, alpha]
        })
        .collect()
}

fn png(path: &Path, width: usize, height: usize, rgba: &[u8]) -> Result<()> {
    let file = BufWriter::new(fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}

fn bitmap(path: &Path, b: &Bitmap, pal: &Palette) -> Result<()> {
    png(path, b.width, b.height, &indexed(&b.pixels, pal, None))
}

/// One PNG per tile and a sheet with 16 tiles per row.
fn tiles(dir: &Path, set: &TileSet, pal: &Palette) -> Result<()> {
    fs::create_dir_all(dir)?;
    let cells: Vec<Vec<u8>> = (0..set.len())
        .map(|i| indexed(set.tile(i).unwrap(), pal, None))
        .collect();
    write_set(dir, &cells)
}

fn objects(dir: &Path, set: &ObjectSet, pal: &Palette) -> Result<()> {
    fs::create_dir_all(dir)?;
    let cells: Vec<Vec<u8>> = (0..set.len())
        .map(|i| {
            let mut pixels = vec![0; TILE * TILE];
            let mut mask = vec![false; TILE * TILE];
            for y in 0..TILE {
                for x in 0..TILE {
                    if let Some(p) = set.pixel(i, x, y) {
                        pixels[y * TILE + x] = p;
                        mask[y * TILE + x] = true;
                    }
                }
            }
            indexed(&pixels, pal, Some(&mask))
        })
        .collect();
    write_set(dir, &cells)
}

fn write_set(dir: &Path, cells: &[Vec<u8>]) -> Result<()> {
    for (i, rgba) in cells.iter().enumerate() {
        png(&dir.join(format!("{i:03}.png")), TILE, TILE, rgba)?;
    }
    let rows = cells.len().div_ceil(PER_ROW);
    let (w, h) = (PER_ROW * (TILE + GAP), rows * (TILE + GAP));
    let mut sheet = vec![0u8; w * h * 4];
    for (i, rgba) in cells.iter().enumerate() {
        let (ox, oy) = ((i % PER_ROW) * (TILE + GAP), (i / PER_ROW) * (TILE + GAP));
        for y in 0..TILE {
            let src = &rgba[y * TILE * 4..(y + 1) * TILE * 4];
            let at = ((oy + y) * w + ox) * 4;
            sheet[at..at + TILE * 4].copy_from_slice(src);
        }
    }
    png(&dir.with_extension("png"), w, h, &sheet)
}

/// 8-bit unsigned mono PCM, as in the 'snd ' resource.
fn wav(path: &Path, s: &Sound) -> Result<()> {
    let n = s.samples.len() as u32;
    let mut out = Vec::with_capacity(44 + s.samples.len());
    out.extend(b"RIFF");
    out.extend((36 + n).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes()); // PCM
    out.extend(1u16.to_le_bytes()); // mono
    out.extend(s.rate.to_le_bytes());
    out.extend(s.rate.to_le_bytes()); // bytes per second
    out.extend(1u16.to_le_bytes()); // block align
    out.extend(8u16.to_le_bytes()); // bits
    out.extend(b"data");
    out.extend(n.to_le_bytes());
    out.extend(&s.samples);
    fs::write(path, out)?;
    Ok(())
}

fn level_text(number: usize, code: Option<&str>, level: &Level) -> String {
    let mut s = format!("# level {number}");
    if let Some(code) = code {
        s += &format!(" code {code}");
    }
    if let Some((x, y)) = level.start() {
        s += &format!(" start {x} {y}");
    }
    s += "\n";
    let grid = |s: &mut String, title: &str, value: &dyn Fn(usize, usize) -> u16| {
        *s += &format!("{title}\n");
        for y in 0..Level::HEIGHT {
            let row: Vec<String> = (0..Level::WIDTH)
                .map(|x| format!("{:3}", value(x, y)))
                .collect();
            *s += &row.join(" ");
            *s += "\n";
        }
    };
    grid(
        &mut s,
        "tiles (translated, as the game uses them)",
        &|x, y| level.tile(x, y),
    );
    grid(&mut s, "floor (translated)", &|x, y| level.floor(x, y));
    grid(&mut s, "file tiles (values in the level file)", &|x, y| {
        level.file_tile(x, y)
    });
    grid(&mut s, "file floor", &|x, y| level.file_floor(x, y) as u16);
    s
}
