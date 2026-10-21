//! The tile and object graphics of the original game (8 bit, 32 × 32).

use crate::error::{Result, corrupt};

pub const TILE_SIZE: usize = 32;
const TILE_BYTES: usize = TILE_SIZE * TILE_SIZE;

/// A rectangular 8-bit image (palette indices, row by row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Bitmap {
    pub fn new(width: usize, height: usize, pixels: Vec<u8>) -> Result<Self> {
        if pixels.len() != width * height {
            return Err(corrupt(format!(
                "bitmap of {width} × {height} has {} bytes",
                pixels.len()
            )));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
}

/// The frame around the game view (a 512 × 364 area, the view is 448 × 320
/// inside it at (32, 32)).
#[derive(Debug, Clone)]
pub struct Frame {
    /// `SHAP 5102`, 448 × 32, above the view.
    pub top: Bitmap,
    /// `SHAP 5103`, 448 × 12, below the view.
    pub bottom: Bitmap,
    /// `SHAP 5104` (first half), 32 × 364, left of the view.
    pub left: Bitmap,
    /// `SHAP 5104` (second half), 32 × 364, right of the view.
    pub right: Bitmap,
}

impl Frame {
    pub const WIDTH: usize = 512;
    pub const HEIGHT: usize = 364;
    pub const VIEW_X: usize = 32;
    pub const VIEW_Y: usize = 32;

    pub fn new(top: Vec<u8>, bottom: Vec<u8>, sides: Vec<u8>) -> Result<Self> {
        let side = 32 * Self::HEIGHT;
        if sides.len() != 2 * side {
            return Err(corrupt(format!("frame sides are {} bytes", sides.len())));
        }
        Ok(Self {
            top: Bitmap::new(448, 32, top)?,
            bottom: Bitmap::new(448, 12, bottom)?,
            left: Bitmap::new(32, Self::HEIGHT, sides[..side].to_vec())?,
            right: Bitmap::new(32, Self::HEIGHT, sides[side..].to_vec())?,
        })
    }
}

/// Opaque 32 × 32 tiles, stored one after another (`SHAP 5101`).
#[derive(Debug, Clone)]
pub struct TileSet {
    pixels: Vec<u8>,
}

impl TileSet {
    pub fn new(pixels: Vec<u8>) -> Result<Self> {
        if !pixels.len().is_multiple_of(TILE_BYTES) {
            return Err(corrupt(format!(
                "tile data is {} bytes, not a multiple of {TILE_BYTES}",
                pixels.len()
            )));
        }
        Ok(Self { pixels })
    }

    pub fn len(&self) -> usize {
        self.pixels.len() / TILE_BYTES
    }

    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }

    /// The 32 × 32 palette indices of a tile, row by row.
    pub fn tile(&self, index: usize) -> Option<&[u8]> {
        self.pixels
            .get(index * TILE_BYTES..(index + 1) * TILE_BYTES)
    }
}

/// Masked 32 × 32 objects (`SHAP 5100`), drawn on top of the floor; also
/// the block letters (32 × 32 and 16 × 16).
///
/// The original blits them a long word at a time: `dst = (dst & mask) | data`.
/// A mask byte of 0xFF therefore means "transparent".
#[derive(Debug, Clone)]
pub struct ObjectSet {
    data: Vec<u8>,
    /// Width and height in pixels.
    size: usize,
}

impl ObjectSet {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        Self::with_size(data, TILE_SIZE)
    }

    /// Objects of `size` × `size` pixels (the small block letters: 16).
    pub fn with_size(data: Vec<u8>, size: usize) -> Result<Self> {
        let bytes = size * size * 2;
        if !data.len().is_multiple_of(bytes) {
            return Err(corrupt(format!(
                "object data is {} bytes, not a multiple of {bytes}",
                data.len()
            )));
        }
        Ok(Self { data, size })
    }

    pub fn len(&self) -> usize {
        self.data.len() / (self.size * self.size * 2)
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The palette index of a pixel, or `None` where the object is transparent.
    pub fn pixel(&self, index: usize, x: usize, y: usize) -> Option<u8> {
        let size = self.size;
        assert!(x < size && y < size);
        let base = index * size * size * 2 + y * 2 * size + (x / 4) * 8 + x % 4;
        let mask = *self.data.get(base)?;
        let value = *self.data.get(base + 4)?;
        // Partial masks do not occur in the game data; treat any set mask
        // bit as transparent like the full 0xFF.
        (mask == 0).then_some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_are_split_by_size() {
        let mut pixels = vec![0; 2 * TILE_BYTES];
        pixels[TILE_BYTES] = 9;
        let tiles = TileSet::new(pixels).unwrap();
        assert_eq!(tiles.len(), 2);
        assert_eq!(tiles.tile(1).unwrap()[0], 9);
        assert!(tiles.tile(2).is_none());
        assert!(TileSet::new(vec![0; 10]).is_err());
    }

    #[test]
    fn object_mask_and_pixels_are_interleaved() {
        let mut data = vec![0xFF; 2 * TILE_BYTES];
        // Row 1, pixels 4..8: second long-word pair of the row.
        let row = 2 * TILE_SIZE;
        data[row + 8..row + 12].copy_from_slice(&[0, 0xFF, 0xFF, 0xFF]);
        data[row + 12..row + 16].copy_from_slice(&[42, 0, 0, 0]);
        let objects = ObjectSet::new(data).unwrap();
        assert_eq!(objects.pixel(0, 4, 1), Some(42));
        assert_eq!(objects.pixel(0, 5, 1), None);
        assert_eq!(objects.pixel(0, 0, 0), None);
        assert_eq!(objects.pixel(1, 0, 0), None);
    }
}
