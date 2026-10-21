//! LZRW1-A decompression (Ross N. Williams, 1991, public domain).
//!
//! The original game stores its levels, graphics, codes and music in 'SHAP'
//! resources ("secret stuff") compressed with LZRW1-A. The original DATA
//! resource still contains the string "LZRW1-A 1.0 22-Jun-1991 Public Domain".

use crate::bytes::{u8_at, u32_at};
use crate::error::{Result, corrupt};

const FLAG_COMPRESSED: u32 = 0;
const FLAG_COPY: u32 = 1;

/// Unpack a 'SHAP' resource: 4 bytes unpacked size (big-endian), then a
/// LZRW1-A block (4 bytes flag + stream).
pub fn unpack_shap(resource: &[u8]) -> Result<Vec<u8>> {
    let size = u32_at(resource, 0)? as usize;
    let out = decompress(&resource[4..])?;
    if out.len() != size {
        return Err(corrupt(format!(
            "unpacked {} bytes, header says {size}",
            out.len()
        )));
    }
    Ok(out)
}

/// Decompress one LZRW1-A block (flag word + stream).
///
/// The stream has a 16-bit control word (little-endian, low bit first) before
/// every 16 items. A 0 bit is a literal byte. A 1 bit is a copy item of two
/// bytes: offset = high nibble of the first byte and the second byte
/// (12 bits), length = low nibble of the first byte + 3.
pub fn decompress(block: &[u8]) -> Result<Vec<u8>> {
    let flag = u32_at(block, 0)?;
    let src = &block[4..];
    match flag {
        FLAG_COPY => return Ok(src.to_vec()),
        FLAG_COMPRESSED => {}
        _ => return Err(corrupt(format!("unknown LZRW1-A flag {flag}"))),
    }
    let mut out = Vec::with_capacity(src.len() * 3);
    let mut i = 0;
    let mut control = 0u16;
    let mut bits = 0;
    while i < src.len() {
        if bits == 0 {
            control = u16::from_le_bytes([u8_at(src, i)?, u8_at(src, i + 1)?]);
            i += 2;
            bits = 16;
            if i >= src.len() {
                break;
            }
        }
        if control & 1 == 1 {
            let first = u8_at(src, i)? as usize;
            let offset = ((first & 0xF0) << 4) | u8_at(src, i + 1)? as usize;
            let len = (first & 0x0F) + 3;
            i += 2;
            if offset == 0 || offset > out.len() {
                return Err(corrupt(format!(
                    "copy offset {offset} at output position {}",
                    out.len()
                )));
            }
            let start = out.len() - offset;
            for k in 0..len {
                out.push(out[start + k]);
            }
        } else {
            out.push(src[i]);
            i += 1;
        }
        control >>= 1;
        bits -= 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_and_copies() {
        // "abcabcabcX": 3 literals, one copy (offset 3, length 6), 1 literal.
        let mut block = FLAG_COMPRESSED.to_be_bytes().to_vec();
        block.extend_from_slice(&0b0000_1000u16.to_le_bytes());
        block.extend_from_slice(b"abc");
        block.extend_from_slice(&[0x03, 0x03]); // length 3+3, offset 3
        block.push(b'X');
        assert_eq!(decompress(&block).unwrap(), b"abcabcabcX");
    }

    #[test]
    fn stored_block_is_copied() {
        let mut block = FLAG_COPY.to_be_bytes().to_vec();
        block.extend_from_slice(b"raw");
        assert_eq!(decompress(&block).unwrap(), b"raw");
    }

    #[test]
    fn copy_before_start_is_an_error() {
        let mut block = FLAG_COMPRESSED.to_be_bytes().to_vec();
        block.extend_from_slice(&1u16.to_le_bytes());
        block.extend_from_slice(&[0x00, 0x05]);
        assert!(decompress(&block).is_err());
    }

    #[test]
    fn shap_size_is_checked() {
        let mut shap = 4u32.to_be_bytes().to_vec();
        shap.extend_from_slice(&FLAG_COPY.to_be_bytes());
        shap.extend_from_slice(b"abc");
        assert!(unpack_shap(&shap).is_err());
        shap[3] = 3;
        assert_eq!(unpack_shap(&shap).unwrap(), b"abc");
    }
}
