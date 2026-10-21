//! Export what the game reads from the original disk into ordinary files,
//! for tests and for looking at:
//!
//! - `graphics/`: tiles, objects, block letters, box tiles and side panel
//!   icons (one PNG each and/or a sheet), the frame around the view and the
//!   pictures, in the colors of the game (objects and letters with
//!   transparency where their mask is clear)
//! - `sounds/`: the sound effects as WAV (8-bit mono, original rate)
//! - `music/`: the music (`SHAP` 200) and the unused earlier version
//!   (`SHAP` 5200) as ProTracker modules, and each as WAV played by the
//!   game's player (8-bit mono, 14162 Hz, once through)
//! - `levels/`: each level as text (translated values the game uses and the
//!   values of the level file) and/or as a picture of the whole level
//! - `codes.txt`, `palette.txt`
//!
//! Usage: scout-extract [OPTIONS] [DISK] OUT_DIR, see `scout-extract --help`.
//! Install with `cargo install --path crates/scout-data`.

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use scout_core::Level;
use scout_core::music::{self, MusicPlayer};
use scout_data::graphics::{ObjectSet, TileSet};
use scout_data::{Bitmap, GameData, Palette};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const TILE: usize = 32;
const PER_ROW: usize = 16;
const GAP: usize = 2;
const USAGE: &str = "usage: scout-extract [OPTIONS] [DISK] OUT_DIR (--help for more)";
const HELP: &str = "\
Export graphics, sounds, music and levels of the original S.C.Out disk.

usage: scout-extract [OPTIONS] [DISK] OUT_DIR

  --only LIST        what to export, comma separated (default: all):
                     graphics, sounds, music, levels, level-images, codes,
                     palette
  --graphics MODE    single (one PNG per item), sheets, or both (default)
  --levels LIST      only these levels (numbers from 1, comma separated)
  -h, --help         this text

DISK is the original SCOUT_1_4.dsk, or the application itself (its
resource fork, e.g. version 1.3.1 unpacked from its archive). Without it:
$SCOUT_DISK, then SCOUT_1_4.dsk in the current folder, then
re/work/SCOUT_1_4.dsk.

Example: scout-extract --only level-images ~/scout-levels";
const PARTS: [&str; 7] = [
    "graphics",
    "sounds",
    "music",
    "levels",
    "level-images",
    "codes",
    "palette",
];

struct Options {
    disk: PathBuf,
    out: PathBuf,
    parts: Vec<String>,
    /// Level numbers from 1; empty = all.
    levels: Vec<usize>,
    single: bool,
    sheets: bool,
}

impl Options {
    fn parse() -> Result<Self> {
        let mut paths = Vec::new();
        let mut parts: Vec<String> = PARTS.iter().map(|p| p.to_string()).collect();
        let (mut single, mut sheets) = (true, true);
        let mut levels = Vec::new();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--only" => {
                    let list = args.next().ok_or(USAGE)?;
                    parts = list.split(',').map(|p| p.trim().to_string()).collect();
                    if let Some(bad) = parts.iter().find(|p| !PARTS.contains(&p.as_str())) {
                        return Err(format!("unknown part {bad:?}; {USAGE}").into());
                    }
                }
                "--levels" => {
                    let list = args.next().ok_or(USAGE)?;
                    for n in list.split(',') {
                        levels.push(n.trim().parse().map_err(|_| format!("bad level {n:?}"))?);
                    }
                }
                "--graphics" => match args.next().as_deref() {
                    Some("single") => (single, sheets) = (true, false),
                    Some("sheets") => (single, sheets) = (false, true),
                    Some("both") => (single, sheets) = (true, true),
                    _ => return Err(USAGE.into()),
                },
                "-h" | "--help" => {
                    println!("{HELP}");
                    std::process::exit(0);
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown option {other}; {USAGE}").into());
                }
                _ => paths.push(PathBuf::from(arg)),
            }
        }
        let (disk, out) = match paths.as_slice() {
            [out] => (find_disk()?, out.clone()),
            [disk, out] => (disk.clone(), out.clone()),
            _ => return Err(USAGE.into()),
        };
        Ok(Self {
            disk,
            out,
            parts,
            levels,
            single,
            sheets,
        })
    }

    fn wants(&self, part: &str) -> bool {
        self.parts.iter().any(|p| p == part)
    }
}

/// A disk image, or an application file: its resource fork (on macOS
/// `FILE/..namedfork/rsrc`) or the file as a raw resource fork.
fn load(path: &Path) -> Result<GameData> {
    match GameData::load(path) {
        Err(scout_data::Error::NotHfs) => {
            let fork = fs::read(path.join("..namedfork/rsrc"))
                .ok()
                .filter(|f| !f.is_empty())
                .map_or_else(|| fs::read(path), Ok)?;
            Ok(GameData::from_resource_fork(&fork)?)
        }
        other => Ok(other?),
    }
}

/// The disk image like the game looks for it.
fn find_disk() -> Result<PathBuf> {
    std::env::var_os("SCOUT_DISK")
        .map(PathBuf::from)
        .into_iter()
        .chain([
            PathBuf::from("SCOUT_1_4.dsk"),
            PathBuf::from("re/work/SCOUT_1_4.dsk"),
        ])
        .find(|p| p.exists())
        .ok_or_else(|| "SCOUT_1_4.dsk not found: pass its path or set SCOUT_DISK".into())
}

fn main() -> Result<()> {
    let opt = Options::parse()?;
    let data = load(&opt.disk)?;
    let pal = &data.game_palette;
    let out = &opt.out;
    let mut done = Vec::new();

    if opt.wants("graphics") {
        let gfx = out.join("graphics");
        let tile_cells = |set: &TileSet| -> Vec<Vec<u8>> {
            (0..set.len())
                .map(|i| indexed(set.tile(i).unwrap(), pal, None))
                .collect()
        };
        let sets = [
            ("tiles", tile_cells(&data.tiles), TILE, TILE),
            ("box", tile_cells(&data.box_tiles), TILE, TILE),
            ("objects", object_cells(&data.objects, pal), TILE, TILE),
            ("font", object_cells(&data.font, pal), TILE, TILE),
            (
                "icons",
                data.icons
                    .iter()
                    .map(|b| indexed(&b.pixels, pal, None))
                    .collect(),
                data.icons[0].width,
                data.icons[0].height,
            ),
        ];
        for (name, cells, w, h) in &sets {
            write_set(&gfx.join(name), cells, *w, *h, &opt)?;
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
        done.push(format!(
            "graphics ({} tiles, {} objects, {} letters, {} icons, {} pictures)",
            data.tiles.len(),
            data.objects.len(),
            data.font.len(),
            data.icons.len(),
            data.pictures.len()
        ));
    }

    if opt.wants("sounds") {
        let sounds = out.join("sounds");
        fs::create_dir_all(&sounds)?;
        for s in &data.sounds {
            wav(
                &sounds.join(format!("snd_{}.wav", s.id)),
                s.rate,
                &s.samples,
            )?;
        }
        done.push(format!("{} sounds", data.sounds.len()));
    }

    if opt.wants("music") {
        let dir = out.join("music");
        fs::create_dir_all(&dir)?;
        let rate = music::RATE.round() as u32;
        // ten minutes at most; the songs take about three
        let max_ticks = (600.0 * music::RATE) as usize / music::TICK_SAMPLES;
        for (name, module) in [
            ("music_200", &data.music),
            ("music_5200_unused", &data.unused_music),
        ] {
            fs::write(dir.join(format!("{name}.mod")), module)?;
            let song = MusicPlayer::render_song(module, &data.music_code, false, max_ticks);
            wav(&dir.join(format!("{name}.wav")), rate, &song)?;
        }
        done.push("music (2 modules)".into());
    }

    let levels = out.join("levels");
    if opt.wants("levels") || opt.wants("level-images") {
        fs::create_dir_all(&levels)?;
    }
    let chosen = |i: usize| opt.levels.is_empty() || opt.levels.contains(&(i + 1));
    let count = (0..data.levels.len()).filter(|&i| chosen(i)).count();
    for (i, level) in data.levels.iter().enumerate() {
        if !chosen(i) {
            continue;
        }
        let name = format!("level_{:03}", i + 1);
        if opt.wants("levels") {
            let code = i.checked_sub(1).map(|c| data.level_codes[c].as_str());
            fs::write(
                levels.join(format!("{name}.txt")),
                level_text(i + 1, code, level),
            )?;
        }
        if opt.wants("level-images") {
            let (w, h) = (Level::WIDTH * TILE, Level::HEIGHT * TILE);
            png(
                &levels.join(format!("{name}.png")),
                w,
                h,
                &level_image(&data, level),
            )?;
        }
    }
    if opt.wants("levels") {
        done.push(format!("{count} levels as text"));
    }
    if opt.wants("level-images") {
        done.push(format!("{count} levels as pictures"));
    }

    if opt.wants("codes") {
        let codes: String = data
            .level_codes
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{} {c}\n", i + 2))
            .collect();
        fs::write(out.join("codes.txt"), codes)?;
        done.push("codes".into());
    }
    if opt.wants("palette") {
        let palette: String = pal
            .colors
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{i} {} {} {}\n", c.r, c.g, c.b))
            .collect();
        fs::write(out.join("palette.txt"), palette)?;
        done.push("palette".into());
    }
    println!("{}: {}", out.display(), done.join(", "));
    Ok(())
}

/// The whole level as the game draws it: tiles from 40 up, otherwise the
/// floor with the object on top (the start box as loaded: 0x108).
fn level_image(data: &GameData, level: &Level) -> Vec<u8> {
    let w = Level::WIDTH * TILE;
    let mut rgba = vec![0u8; w * Level::HEIGHT * TILE * 4];
    let mut put = |x: usize, y: usize, c: [u8; 4]| {
        let at = (y * w + x) * 4;
        rgba[at..at + 4].copy_from_slice(&c);
    };
    let color = |i: u8| {
        let c = data.game_palette.colors[i as usize];
        [c.r, c.g, c.b, 255]
    };
    const MISSING: [u8; 4] = [255, 0, 255, 255];
    for by in 0..Level::HEIGHT {
        for bx in 0..Level::WIDTH {
            let value = level.tile(bx, by) as usize;
            let base = if value >= Level::FIRST_SOLID_TILE as usize {
                value
            } else {
                level.floor(bx, by) as usize
            };
            let tile = data.tiles.tile(base);
            for y in 0..TILE {
                for x in 0..TILE {
                    let c = tile.map_or(MISSING, |t| color(t[y * TILE + x]));
                    put(bx * TILE + x, by * TILE + y, c);
                    if value != 0
                        && value < Level::FIRST_SOLID_TILE as usize
                        && let Some(p) = data.objects.pixel(value, x, y)
                    {
                        put(bx * TILE + x, by * TILE + y, color(p));
                    }
                }
            }
        }
    }
    rgba
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

fn object_cells(set: &ObjectSet, pal: &Palette) -> Vec<Vec<u8>> {
    (0..set.len())
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
        .collect()
}

/// One PNG per item in `dir` and/or a sheet `dir.png` with 16 per row.
fn write_set(dir: &Path, cells: &[Vec<u8>], cw: usize, ch: usize, opt: &Options) -> Result<()> {
    if opt.single {
        fs::create_dir_all(dir)?;
        for (i, rgba) in cells.iter().enumerate() {
            png(&dir.join(format!("{i:03}.png")), cw, ch, rgba)?;
        }
    }
    if !opt.sheets {
        return Ok(());
    }
    if let Some(parent) = dir.parent() {
        fs::create_dir_all(parent)?;
    }
    let rows = cells.len().div_ceil(PER_ROW);
    let (w, h) = (PER_ROW * (cw + GAP), rows * (ch + GAP));
    let mut sheet = vec![0u8; w * h * 4];
    for (i, rgba) in cells.iter().enumerate() {
        let (ox, oy) = ((i % PER_ROW) * (cw + GAP), (i / PER_ROW) * (ch + GAP));
        for y in 0..ch {
            let src = &rgba[y * cw * 4..(y + 1) * cw * 4];
            let at = ((oy + y) * w + ox) * 4;
            sheet[at..at + cw * 4].copy_from_slice(src);
        }
    }
    png(&dir.with_extension("png"), w, h, &sheet)
}

/// 8-bit unsigned mono PCM, as in the 'snd ' resource.
fn wav(path: &Path, rate: u32, samples: &[u8]) -> Result<()> {
    let n = samples.len() as u32;
    let mut out = Vec::with_capacity(44 + samples.len());
    out.extend(b"RIFF");
    out.extend((36 + n).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes()); // PCM
    out.extend(1u16.to_le_bytes()); // mono
    out.extend(rate.to_le_bytes());
    out.extend(rate.to_le_bytes()); // bytes per second
    out.extend(1u16.to_le_bytes()); // block align
    out.extend(8u16.to_le_bytes()); // bits
    out.extend(b"data");
    out.extend(n.to_le_bytes());
    out.extend(samples);
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
