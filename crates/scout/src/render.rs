//! Software rendering of a level into an RGB image, like the original:
//! level values are translated to Mac graphics indices; indices from 40 up
//! are opaque tiles, indices below are objects drawn with their mask on top
//! of the floor tile.

use scout_core::Level;
use scout_data::graphics::TILE_SIZE;
use scout_data::{GameData, Palette, translate};

pub const WIDTH: usize = Level::WIDTH * TILE_SIZE;
pub const HEIGHT: usize = Level::HEIGHT * TILE_SIZE;
/// Palette index used where the game data has no graphic for a value.
const MISSING: u8 = 0;
/// Mac graphics indices from here on are opaque tiles, below are objects.
const FIRST_SOLID_GRAPHIC: u16 = 40;

/// Compose the palette indices of a whole level (`WIDTH` × `HEIGHT`).
pub fn level_indices(game: &GameData, level: &Level) -> Vec<u8> {
    let mut out = vec![MISSING; WIDTH * HEIGHT];
    for by in 0..Level::HEIGHT {
        for bx in 0..Level::WIDTH {
            let value = translate::tile(level.tile(bx, by));
            let (base, object) = if value >= FIRST_SOLID_GRAPHIC {
                (value as usize, None)
            } else {
                (
                    translate::floor(level.floor(bx, by)) as usize,
                    (value != 0).then_some(value as usize),
                )
            };
            for y in 0..TILE_SIZE {
                let row = (by * TILE_SIZE + y) * WIDTH + bx * TILE_SIZE;
                if let Some(tile) = game.tiles.tile(base) {
                    out[row..row + TILE_SIZE]
                        .copy_from_slice(&tile[y * TILE_SIZE..(y + 1) * TILE_SIZE]);
                }
                if let Some(object) = object.filter(|&o| o < game.objects.len()) {
                    for x in 0..TILE_SIZE {
                        if let Some(p) = game.objects.pixel(object, x, y) {
                            out[row + x] = p;
                        }
                    }
                }
            }
        }
    }
    out
}

/// Convert palette indices to packed RGB24 rows with the given pitch.
pub fn to_rgb24(indices: &[u8], palette: &Palette, out: &mut [u8], pitch: usize) {
    for (y, row) in indices.chunks_exact(WIDTH).enumerate() {
        let line = &mut out[y * pitch..y * pitch + WIDTH * 3];
        for (px, &i) in line.chunks_exact_mut(3).zip(row) {
            let c = palette.colors[i as usize];
            px.copy_from_slice(&[c.r, c.g, c.b]);
        }
    }
}

/// Write an RGB24 image (rows of `WIDTH * 3` bytes) as an uncompressed BMP.
pub fn write_bmp(path: &std::path::Path, rgb: &[u8]) -> std::io::Result<()> {
    let row = WIDTH * 3; // already a multiple of 4
    let size = 54 + row * HEIGHT;
    let mut bmp = Vec::with_capacity(size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&(WIDTH as i32).to_le_bytes());
    bmp.extend_from_slice(&(HEIGHT as i32).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&[0; 24]);
    for y in (0..HEIGHT).rev() {
        for px in rgb[y * row..(y + 1) * row].chunks_exact(3) {
            bmp.extend_from_slice(&[px[2], px[1], px[0]]);
        }
    }
    std::fs::write(path, bmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scout_data::palette::Rgb;

    #[test]
    fn rgb_conversion_uses_the_palette() {
        let mut palette = Palette {
            colors: [Rgb::default(); 256],
        };
        palette.colors[3] = Rgb { r: 1, g: 2, b: 3 };
        let mut indices = vec![0; WIDTH * HEIGHT];
        indices[WIDTH + 1] = 3;
        let mut rgb = vec![0; WIDTH * 3 * HEIGHT];
        to_rgb24(&indices, &palette, &mut rgb, WIDTH * 3);
        assert_eq!(&rgb[WIDTH * 3 + 3..WIDTH * 3 + 6], &[1, 2, 3]);
        assert_eq!(&rgb[0..3], &[0, 0, 0]);
    }
}
