//! Bounds-checked big-endian reads.

use crate::error::{Result, corrupt};

pub(crate) fn slice(data: &[u8], offset: usize, len: usize) -> Result<&[u8]> {
    offset
        .checked_add(len)
        .and_then(|end| data.get(offset..end))
        .ok_or_else(|| {
            corrupt(format!(
                "read of {len} bytes at {offset} is past the end ({})",
                data.len()
            ))
        })
}

pub(crate) fn u8_at(data: &[u8], offset: usize) -> Result<u8> {
    Ok(slice(data, offset, 1)?[0])
}

pub(crate) fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    let b = slice(data, offset, 2)?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}

pub(crate) fn i16_at(data: &[u8], offset: usize) -> Result<i16> {
    Ok(u16_at(data, offset)? as i16)
}

pub(crate) fn u32_at(data: &[u8], offset: usize) -> Result<u32> {
    let b = slice(data, offset, 4)?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Decode a Mac Roman string; only ASCII is needed for this game.
pub(crate) fn mac_roman(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| if b < 0x80 { b as char } else { '\u{FFFD}' })
        .collect()
}
