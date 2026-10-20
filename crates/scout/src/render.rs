//! Software rendering into palette indices, like the original: tile values
//! (Mac graphics indices, see `scout_core::Level`) from 40 up are opaque
//! tiles, values below are objects drawn with their mask on top of the
//! floor tile.

use scout_core::{Game, Level};
use scout_data::graphics::TILE_SIZE;
use scout_data::{Bitmap, Frame, GameData, Palette};

/// Palette index used where the game data has no graphic for a value.
const MISSING: u8 = 0;
/// Black in the game palette (screen index 255 = palette entry 1).
const BLACK: u8 = 255;

pub const SCREEN_WIDTH: usize = 640;
pub const SCREEN_HEIGHT: usize = 480;
/// Position of the frame on the 640 × 480 screen of the original.
const FRAME_X: usize = 64;
const FRAME_Y: usize = 68;
const VIEW_X: usize = FRAME_X + Frame::VIEW_X;
const VIEW_Y: usize = FRAME_Y + Frame::VIEW_Y;

/// A projectile as the original draws it (CODE 8 0x364): 4 × 4 pixels with
/// the top left corner at its position. The original takes the colors of
/// the top and bottom row from the bytes at addresses 0x64 and 0x65 (a bug:
/// `move.b $64.w` instead of `#$64`), part of an interrupt vector that
/// depends on the Mac; Basilisk II has 0x01 (grey) and 0x00 (white) there.
/// `None` is transparent.
const PROJECTILE: [[Option<u8>; 4]; 4] = {
    const TOP: Option<u8> = Some(0x01);
    const BOTTOM: Option<u8> = Some(0x00);
    const ORANGE: Option<u8> = Some(0x64);
    const YELLOW: Option<u8> = Some(0x65);
    [
        [None, TOP, TOP, None],
        [ORANGE, YELLOW, YELLOW, YELLOW],
        [ORANGE, YELLOW, YELLOW, YELLOW],
        [None, BOTTOM, BOTTOM, None],
    ]
};

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
        data: &GameData,
        level: &Level,
        bx: usize,
        by: usize,
        x: usize,
        y: usize,
    ) {
        let value = level.tile(bx, by);
        let (base, object) = if value >= Level::FIRST_SOLID_TILE {
            (value as usize, None)
        } else {
            (
                level.floor(bx, by) as usize,
                (value != 0).then_some(value as usize),
            )
        };
        for row in 0..TILE_SIZE {
            let at = (y + row) * self.width + x;
            match data.tiles.tile(base) {
                Some(tile) => self.pixels[at..at + TILE_SIZE]
                    .copy_from_slice(&tile[row * TILE_SIZE..(row + 1) * TILE_SIZE]),
                None => self.pixels[at..at + TILE_SIZE].fill(MISSING),
            }
        }
        if let Some(object) = object {
            self.draw_object(data, object, x as i32, y as i32);
        }
    }

    /// The projectiles the original has on screen (drawing state 2 or 3),
    /// clipped to the view.
    fn draw_projectiles(&mut self, game: &Game, (vx, vy): (usize, usize)) {
        let (w, h) = (
            (Level::VIEW_WIDTH * TILE_SIZE) as i32,
            (Level::VIEW_HEIGHT * TILE_SIZE) as i32,
        );
        for p in game.world.projectiles.iter().filter(|p| p.draw > 1) {
            let left = p.x - (vx * TILE_SIZE) as i32;
            let top = p.y - (vy * TILE_SIZE) as i32;
            for (dy, row) in PROJECTILE.iter().enumerate() {
                for (dx, color) in row.iter().enumerate() {
                    let (x, y) = (left + dx as i32, top + dy as i32);
                    if let Some(c) = color
                        && (0..w).contains(&x)
                        && (0..h).contains(&y)
                    {
                        self.pixels[(VIEW_Y + y as usize) * self.width + VIEW_X + x as usize] = *c;
                    }
                }
            }
        }
    }

    /// Draw a masked object at pixel position (x, y), clipped to the canvas.
    fn draw_object(&mut self, data: &GameData, object: usize, x: i32, y: i32) {
        if object >= data.objects.len() {
            return;
        }
        for row in 0..TILE_SIZE {
            for col in 0..TILE_SIZE {
                let (px, py) = (x + col as i32, y + row as i32);
                if px < 0 || py < 0 || px as usize >= self.width || py as usize >= self.height {
                    continue;
                }
                if let Some(p) = data.objects.pixel(object, col, row) {
                    self.pixels[py as usize * self.width + px as usize] = p;
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

/// The whole level (40 × 24 boxes) without frame, with the player.
pub fn level_map(data: &GameData, game: &Game) -> Canvas {
    let mut canvas = Canvas::new(Level::WIDTH * TILE_SIZE, Level::HEIGHT * TILE_SIZE, MISSING);
    for by in 0..Level::HEIGHT {
        for bx in 0..Level::WIDTH {
            canvas.draw_box(data, &game.level, bx, by, bx * TILE_SIZE, by * TILE_SIZE);
        }
    }
    let p = &game.player;
    let (ox, oy) = p.offset();
    let x = (p.x * TILE_SIZE) as i32 + ox;
    let y = (p.y * TILE_SIZE) as i32 + oy;
    if game.world.player_visible {
        canvas.draw_object(data, p.sprite(), x, y);
    }
    canvas
}

/// The 640 × 480 game screen as the original shows it (without the Mac
/// menu bar): frame, the 14 × 10 view and the player.
pub fn game_screen(data: &GameData, game: &Game) -> Canvas {
    let mut canvas = Canvas::new(SCREEN_WIDTH, SCREEN_HEIGHT, BLACK);
    let frame = &data.frame;
    canvas.blit(&frame.top, VIEW_X, FRAME_Y);
    canvas.blit(&frame.left, FRAME_X, FRAME_Y);
    canvas.blit(
        &frame.right,
        VIEW_X + Level::VIEW_WIDTH * TILE_SIZE,
        FRAME_Y,
    );
    canvas.blit(
        &frame.bottom,
        VIEW_X,
        VIEW_Y + Level::VIEW_HEIGHT * TILE_SIZE,
    );

    let (vx, vy) = game.shown_view();
    for by in 0..Level::VIEW_HEIGHT {
        for bx in 0..Level::VIEW_WIDTH {
            let (x, y) = (VIEW_X + bx * TILE_SIZE, VIEW_Y + by * TILE_SIZE);
            canvas.draw_box(data, &game.level, vx + bx, vy + by, x, y);
        }
    }
    let p = &game.player;
    let (ox, oy) = p.offset();
    let x = VIEW_X as i32 + (p.x as i32 - vx as i32) * TILE_SIZE as i32 + ox;
    let y = VIEW_Y as i32 + (p.y as i32 - vy as i32) * TILE_SIZE as i32 + oy;
    if game.world.player_visible {
        canvas.draw_object(data, p.sprite(), x, y);
    }
    canvas.draw_projectiles(game, (vx, vy));
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
