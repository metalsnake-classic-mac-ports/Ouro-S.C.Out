//! Software rendering into palette indices, like the original: level values
//! are translated to Mac graphics indices; indices from 40 up are opaque
//! tiles, indices below are objects drawn with their mask on top of the
//! floor tile.

use scout_core::Level;
use scout_data::graphics::TILE_SIZE;
use scout_data::{Bitmap, Frame, GameData, Palette, translate};

/// Mac graphics indices from here on are opaque tiles, below are objects.
const FIRST_SOLID_GRAPHIC: u16 = 40;
/// Palette index used where the game data has no graphic for a value.
const MISSING: u8 = 0;
/// Black in the game palette (screen index 255 = palette entry 1).
const BLACK: u8 = 255;
/// The object the original shows on the start box when a level begins.
/// (Seen in a screenshot of the original; the player animation is not
/// decoded yet.)
const START_OBJECT: usize = 15;

pub const SCREEN_WIDTH: usize = 640;
pub const SCREEN_HEIGHT: usize = 480;
/// Position of the frame on the 640 × 480 screen of the original.
const FRAME_X: usize = 64;
const FRAME_Y: usize = 68;

/// An image of palette indices.
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: usize, height: usize, fill: u8) -> Self {
        Self {
            width,
            height,
            pixels: vec![fill; width * height],
        }
    }

    fn blit(&mut self, bitmap: &Bitmap, x: usize, y: usize) {
        for row in 0..bitmap.height {
            let src = &bitmap.pixels[row * bitmap.width..(row + 1) * bitmap.width];
            let at = (y + row) * self.width + x;
            self.pixels[at..at + bitmap.width].copy_from_slice(src);
        }
    }

    /// Draw one level box at pixel position (x, y).
    fn draw_box(
        &mut self,
        game: &GameData,
        level: &Level,
        bx: usize,
        by: usize,
        x: usize,
        y: usize,
    ) {
        let raw = level.tile(bx, by);
        let value = if level.is_start_marker(bx, by) {
            0
        } else {
            translate::tile(raw)
        };
        let (base, object) = if value >= FIRST_SOLID_GRAPHIC {
            (value as usize, None)
        } else {
            (
                translate::floor(level.floor(bx, by)) as usize,
                (value != 0).then_some(value as usize),
            )
        };
        for row in 0..TILE_SIZE {
            let at = (y + row) * self.width + x;
            match game.tiles.tile(base) {
                Some(tile) => self.pixels[at..at + TILE_SIZE]
                    .copy_from_slice(&tile[row * TILE_SIZE..(row + 1) * TILE_SIZE]),
                None => self.pixels[at..at + TILE_SIZE].fill(MISSING),
            }
        }
        if let Some(object) = object {
            self.draw_object(game, object, x, y);
        }
    }

    fn draw_object(&mut self, game: &GameData, object: usize, x: usize, y: usize) {
        if object >= game.objects.len() {
            return;
        }
        for row in 0..TILE_SIZE {
            for col in 0..TILE_SIZE {
                if let Some(p) = game.objects.pixel(object, col, row) {
                    self.pixels[(y + row) * self.width + x + col] = p;
                }
            }
        }
    }

    /// Convert to packed RGB24 rows with the given pitch.
    pub fn to_rgb24(&self, palette: &Palette, out: &mut [u8], pitch: usize) {
        for (y, row) in self.pixels.chunks_exact(self.width).enumerate() {
            let line = &mut out[y * pitch..y * pitch + self.width * 3];
            for (px, &i) in line.as_chunks_mut::<3>().0.iter_mut().zip(row) {
                let c = palette.colors[i as usize];
                *px = [c.r, c.g, c.b];
            }
        }
    }
}

/// The whole level (40 × 24 boxes) without frame.
pub fn level_map(game: &GameData, level: &Level) -> Canvas {
    let mut canvas = Canvas::new(Level::WIDTH * TILE_SIZE, Level::HEIGHT * TILE_SIZE, MISSING);
    for by in 0..Level::HEIGHT {
        for bx in 0..Level::WIDTH {
            canvas.draw_box(game, level, bx, by, bx * TILE_SIZE, by * TILE_SIZE);
        }
    }
    if let Some((sx, sy)) = level.start() {
        canvas.draw_object(game, START_OBJECT, sx * TILE_SIZE, sy * TILE_SIZE);
    }
    canvas
}

/// The 640 × 480 game screen at the start of a level, as the original
/// shows it (without the Mac menu bar).
pub fn start_screen(game: &GameData, level: &Level) -> Canvas {
    let mut canvas = Canvas::new(SCREEN_WIDTH, SCREEN_HEIGHT, BLACK);
    let frame = &game.frame;
    let view_x = FRAME_X + Frame::VIEW_X;
    let view_y = FRAME_Y + Frame::VIEW_Y;
    canvas.blit(&frame.top, view_x, FRAME_Y);
    canvas.blit(&frame.left, FRAME_X, FRAME_Y);
    canvas.blit(
        &frame.right,
        view_x + Level::VIEW_WIDTH * TILE_SIZE,
        FRAME_Y,
    );
    canvas.blit(
        &frame.bottom,
        view_x,
        view_y + Level::VIEW_HEIGHT * TILE_SIZE,
    );

    let (vx, vy) = level.start_view();
    for by in 0..Level::VIEW_HEIGHT {
        for bx in 0..Level::VIEW_WIDTH {
            let (x, y) = (view_x + bx * TILE_SIZE, view_y + by * TILE_SIZE);
            canvas.draw_box(game, level, vx + bx, vy + by, x, y);
        }
    }
    if let Some((sx, sy)) = level.start() {
        let (x, y) = (
            view_x + (sx - vx) * TILE_SIZE,
            view_y + (sy - vy) * TILE_SIZE,
        );
        canvas.draw_object(game, START_OBJECT, x, y);
    }
    canvas
}

/// Write an RGB24 image (rows of `width * 3` bytes) as an uncompressed BMP.
pub fn write_bmp(
    path: &std::path::Path,
    width: usize,
    height: usize,
    rgb: &[u8],
) -> std::io::Result<()> {
    let row = width * 3;
    let padded = row.next_multiple_of(4);
    let size = 54 + padded * height;
    let mut bmp = Vec::with_capacity(size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&(width as i32).to_le_bytes());
    bmp.extend_from_slice(&(height as i32).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&[0; 24]);
    for y in (0..height).rev() {
        for px in rgb[y * row..(y + 1) * row].as_chunks::<3>().0 {
            bmp.extend_from_slice(&[px[2], px[1], px[0]]);
        }
        bmp.resize(bmp.len() + padded - row, 0);
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
        let mut canvas = Canvas::new(4, 2, 0);
        canvas.pixels[4 + 1] = 3;
        let mut rgb = vec![0; 4 * 3 * 2];
        canvas.to_rgb24(&palette, &mut rgb, 4 * 3);
        assert_eq!(&rgb[12 + 3..12 + 6], &[1, 2, 3]);
        assert_eq!(&rgb[0..3], &[0, 0, 0]);
    }

    #[test]
    fn blit_copies_rows() {
        let mut canvas = Canvas::new(4, 3, 0);
        canvas.blit(&Bitmap::new(2, 2, vec![1, 2, 3, 4]).unwrap(), 1, 1);
        assert_eq!(canvas.pixels, [0, 0, 0, 0, 0, 1, 2, 0, 0, 3, 4, 0]);
    }
}
