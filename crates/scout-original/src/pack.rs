//! Writes the data pack (see `scout_data::pack`).

use std::io::{Cursor, Write};

use scout_core::Level;
use scout_data::pack::{FORMAT, TILE};
use scout_data::{Bitmap, GameData, Music, Palette};
use zip::write::SimpleFileOptions;

use crate::error::{Result, corrupt};

/// The data as a pack (the bytes of the zip file, see `scout_data::pack`).
pub fn write(game: &GameData) -> Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut put = |name: &str, data: &[u8]| -> Result<()> {
        zip.start_file(name, SimpleFileOptions::default())
            .map_err(|e| corrupt(e.to_string()))?;
        zip.write_all(data).map_err(|e| corrupt(e.to_string()))
    };
    put(
        "pack.txt",
        format!("{FORMAT}\nThe data of S.C.Out (Kalisto, 1993), converted from the original.\n")
            .as_bytes(),
    )?;
    let palette = &game.game_palette;
    put("palette.gpl", gpl(palette).as_bytes())?;
    for i in 0..game.tiles.len() {
        let pixels = game.tiles.tile(i).unwrap();
        put(
            &format!("graphics/tiles/{i:03}.png"),
            &png_indexed(pixels, TILE, TILE, palette, None),
        )?;
    }
    for (dir, set) in [
        ("objects", &game.objects),
        ("font", &game.font),
        ("small_font", &game.small_font),
    ] {
        for i in 0..set.len() {
            let n = set.size();
            let cells: Vec<Option<u8>> = (0..n * n).map(|p| set.pixel(i, p % n, p / n)).collect();
            let pixels: Vec<u8> = cells.iter().map(|c| c.unwrap_or(0)).collect();
            let mask: Vec<bool> = cells.iter().map(Option::is_some).collect();
            put(
                &format!("graphics/{dir}/{i:02}.png"),
                &png_indexed(&pixels, n, n, palette, Some(&mask)),
            )?;
        }
    }
    for i in 0..game.box_tiles.len() {
        let pixels = game.box_tiles.tile(i).unwrap();
        put(
            &format!("graphics/box/{i}.png"),
            &png_indexed(pixels, TILE, TILE, palette, None),
        )?;
    }
    for (i, icon) in game.icons.iter().enumerate() {
        put(
            &format!("graphics/icons/{i:02}.png"),
            &png_bitmap(icon, palette),
        )?;
    }
    let frame = &game.frame;
    for (name, b) in [
        ("top", &frame.top),
        ("bottom", &frame.bottom),
        ("left", &frame.left),
        ("right", &frame.right),
    ] {
        put(
            &format!("graphics/frame/{name}.png"),
            &png_bitmap(b, palette),
        )?;
    }
    for p in &game.pictures {
        let img = &p.image;
        put(
            &format!("pictures/{}.png", p.id),
            &png_indexed(&img.pixels, img.width, img.height, &img.palette, None),
        )?;
    }
    for s in &game.sounds {
        put(&format!("sounds/{}.wav", s.id), &wav(s.rate, &s.samples))?;
    }
    put("music/music.flac", &flac(&game.music)?)?;
    for (i, level) in game.levels.iter().enumerate() {
        put(
            &format!("levels/{:03}.txt", i + 1),
            level_text(i + 1, level).as_bytes(),
        )?;
    }
    put(
        "levels/codes.txt",
        (game.level_codes.join("\n") + "\n").as_bytes(),
    )?;
    let cursor = zip.finish().map_err(|e| corrupt(e.to_string()))?;
    Ok(cursor.into_inner())
}

fn png_bitmap(b: &Bitmap, palette: &Palette) -> Vec<u8> {
    png_indexed(&b.pixels, b.width, b.height, palette, None)
}

/// An RGBA PNG of palette indices (transparent where `mask` is false).
fn png_indexed(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    mask: Option<&[bool]>,
) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);
    for (i, &p) in pixels.iter().enumerate() {
        let c = palette.colors[p as usize];
        let opaque = mask.is_none_or(|m| m[i]);
        rgba.extend([c.r, c.g, c.b, if opaque { 255 } else { 0 }]);
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header");
        writer.write_image_data(&rgba).expect("PNG data");
    }
    out
}

fn gpl(palette: &Palette) -> String {
    let mut s = String::from("GIMP Palette\nName: S.C.Out game screen\nColumns: 16\n#\n");
    for (i, c) in palette.colors.iter().enumerate() {
        s += &format!("{:3} {:3} {:3}\tIndex {i}\n", c.r, c.g, c.b);
    }
    s
}

/// 8-bit unsigned mono WAV.
fn wav(rate: u32, samples: &[u8]) -> Vec<u8> {
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
    out
}

fn level_text(number: usize, level: &Level) -> String {
    let mut s = format!("# level {number}: the values of the level file\n# tiles\n");
    let grid = |s: &mut String, value: &dyn Fn(usize, usize) -> u16| {
        for y in 0..Level::HEIGHT {
            let row: Vec<String> = (0..Level::WIDTH)
                .map(|x| format!("{:3}", value(x, y)))
                .collect();
            *s += &row.join(" ");
            *s += "\n";
        }
    };
    grid(&mut s, &|x, y| level.file_tile(x, y));
    s += "# floor\n";
    grid(&mut s, &|x, y| level.file_floor(x, y) as u16);
    s
}

/// FLAC, mono: 8 bits if the low byte is always 0 (the original's music),
/// else 16.
fn flac(music: &Music) -> Result<Vec<u8>> {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;
    let bits = if music.samples.iter().all(|s| s & 0xFF == 0) {
        8
    } else {
        16
    };
    let samples: Vec<i32> = music
        .samples
        .iter()
        .map(|&s| (s as i32) >> (16 - bits))
        .collect();
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|e| corrupt(format!("FLAC: {e:?}")))?;
    let source = flacenc::source::MemSource::from_samples(&samples, 1, bits, music.rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| corrupt(format!("FLAC: {e:?}")))?;
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| corrupt(format!("FLAC: {e:?}")))?;
    Ok(sink.as_slice().to_vec())
}
