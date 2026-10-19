//! Decoder for the 8-bit PICT v2 images of the game (PackBits pixmaps with
//! an embedded color table).

use crate::bytes::{i16_at, slice, u8_at, u16_at};
use crate::error::{Result, corrupt};
use crate::palette::Palette;

const PACK_BITS_RECT: u8 = 0x98;
const PACK_BITS_RGN: u8 = 0x99;
const DIRECT_BITS_RECT: u8 = 0x9A;
const DIRECT_BITS_RGN: u8 = 0x9B;

#[derive(Debug, Clone)]
pub struct IndexedImage {
    pub width: usize,
    pub height: usize,
    /// One palette index per pixel, row by row.
    pub pixels: Vec<u8>,
    pub palette: Palette,
}

/// Decode the first 8-bit pixmap opcode of a PICT resource.
pub fn decode(pict: &[u8]) -> Result<IndexedImage> {
    let (opcode_at, pixmap) = find_pixmap(pict)?;
    let row_bytes = (u16_at(pict, pixmap)? & 0x3FFF) as usize;
    let top = i16_at(pict, pixmap + 2)?;
    let left = i16_at(pict, pixmap + 4)?;
    let bottom = i16_at(pict, pixmap + 6)?;
    let right = i16_at(pict, pixmap + 8)?;
    let width = (right - left).max(0) as usize;
    let height = (bottom - top).max(0) as usize;
    if width > row_bytes {
        return Err(corrupt("PICT pixmap is wider than its rows"));
    }
    let (palette, table_len) = Palette::from_color_table(&pict[pixmap + 46..])?;
    // After the color table: source rect, destination rect, transfer mode.
    let mut pos = pixmap + 46 + table_len + 8 + 8 + 2;
    if u8_at(pict, opcode_at + 1)? == PACK_BITS_RGN {
        pos += u16_at(pict, pos)? as usize; // mask region
    }
    let mut pixels = Vec::with_capacity(width * height);
    for _ in 0..height {
        let packed_len = if row_bytes > 250 {
            let n = u16_at(pict, pos)? as usize;
            pos += 2;
            n
        } else {
            let n = u8_at(pict, pos)? as usize;
            pos += 1;
            n
        };
        let row = unpack_bits(slice(pict, pos, packed_len)?, row_bytes)?;
        pos += packed_len;
        pixels.extend_from_slice(&row[..width]);
    }
    Ok(IndexedImage {
        width,
        height,
        pixels,
        palette,
    })
}

/// Find the first PackBits pixmap opcode with an 8-bit pixmap.
/// Returns the opcode position and the pixmap position.
fn find_pixmap(pict: &[u8]) -> Result<(usize, usize)> {
    for at in 0..pict.len().saturating_sub(2) {
        let op = pict[at + 1];
        if pict[at] != 0 || !(PACK_BITS_RECT..=DIRECT_BITS_RGN).contains(&op) {
            continue;
        }
        let pixmap = at
            + 2
            + if matches!(op, DIRECT_BITS_RECT | DIRECT_BITS_RGN) {
                4
            } else {
                0
            };
        let Ok(row_bytes) = u16_at(pict, pixmap) else {
            continue;
        };
        let Ok(pixel_size) = u16_at(pict, pixmap + 28) else {
            continue;
        };
        let Ok(table_size) = u16_at(pict, pixmap + 46 + 6) else {
            continue;
        };
        if row_bytes & 0x8000 != 0 && pixel_size == 8 && table_size < 256 {
            return Ok((at, pixmap));
        }
    }
    Err(corrupt("no 8-bit pixmap in PICT"))
}

/// Apple PackBits run-length decoding of one row.
pub fn unpack_bits(src: &[u8], len: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while out.len() < len {
        let n = u8_at(src, i)?;
        i += 1;
        match n {
            0..=127 => {
                out.extend_from_slice(slice(src, i, n as usize + 1)?);
                i += n as usize + 1;
            }
            128 => {}
            _ => {
                out.extend(std::iter::repeat_n(u8_at(src, i)?, 257 - n as usize));
                i += 1;
            }
        }
    }
    out.truncate(len);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_bits_literal_and_run() {
        // 3 literals, then 4 × 0x07.
        let packed = [2, 1, 2, 3, 0xFD, 7];
        assert_eq!(unpack_bits(&packed, 7).unwrap(), [1, 2, 3, 7, 7, 7, 7]);
    }

    #[test]
    fn pack_bits_too_short_is_an_error() {
        assert!(unpack_bits(&[5, 1, 2], 6).is_err());
    }
}
