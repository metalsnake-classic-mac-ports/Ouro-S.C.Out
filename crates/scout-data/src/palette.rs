//! 256-color palettes: Mac 'pltt' resources and QuickDraw color tables.

use crate::bytes::{u16_at, u32_at};
use crate::error::{Result, corrupt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub colors: [Rgb; 256],
}

impl Palette {
    /// Parse a 'pltt' resource. Entries beyond the stored count stay black.
    pub fn from_pltt(data: &[u8]) -> Result<Self> {
        let count = u16_at(data, 0)? as usize;
        if count > 256 {
            return Err(corrupt(format!("pltt has {count} entries")));
        }
        let mut colors = [Rgb::default(); 256];
        for (i, color) in colors.iter_mut().take(count).enumerate() {
            *color = rgb48(data, 16 + i * 16)?;
        }
        Ok(Self { colors })
    }

    /// Parse a QuickDraw ColorTable (as embedded in PICT pixmaps).
    /// Returns the palette and the size of the table in bytes.
    pub fn from_color_table(data: &[u8]) -> Result<(Self, usize)> {
        let _seed = u32_at(data, 0)?;
        let flags = u16_at(data, 4)?;
        let count = u16_at(data, 6)? as usize + 1;
        if count > 256 {
            return Err(corrupt(format!("color table has {count} entries")));
        }
        let device_table = flags & 0x8000 != 0;
        let mut colors = [Rgb::default(); 256];
        for k in 0..count {
            let entry = 8 + k * 8;
            // In a device color table the value field is ignored; the
            // position is the index.
            let index = if device_table {
                k
            } else {
                u16_at(data, entry)? as usize & 0xFF
            };
            colors[index] = rgb48(data, entry + 2)?;
        }
        Ok((Self { colors }, 8 + count * 8))
    }
}

fn rgb48(data: &[u8], at: usize) -> Result<Rgb> {
    Ok(Rgb {
        r: (u16_at(data, at)? >> 8) as u8,
        g: (u16_at(data, at + 2)? >> 8) as u8,
        b: (u16_at(data, at + 4)? >> 8) as u8,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pltt_entries() {
        let mut data = vec![0u8; 16];
        data[1] = 2;
        for (r, g, b) in [(0xFFFFu16, 0x8000u16, 0u16), (0x1234, 0, 0xFF00)] {
            for v in [r, g, b] {
                data.extend_from_slice(&v.to_be_bytes());
            }
            data.extend_from_slice(&[0; 10]);
        }
        let p = Palette::from_pltt(&data).unwrap();
        assert_eq!(
            p.colors[0],
            Rgb {
                r: 0xFF,
                g: 0x80,
                b: 0
            }
        );
        assert_eq!(
            p.colors[1],
            Rgb {
                r: 0x12,
                g: 0,
                b: 0xFF
            }
        );
        assert_eq!(p.colors[2], Rgb::default());
    }

    #[test]
    fn color_table_uses_values_unless_device_table() {
        let mut ct = vec![0, 0, 0, 0, 0, 0, 0, 0]; // seed, flags, size-1 = 0
        ct.extend_from_slice(&5u16.to_be_bytes());
        ct.extend_from_slice(&[0xFF, 0, 0, 0, 0, 0]);
        let (p, len) = Palette::from_color_table(&ct).unwrap();
        assert_eq!(len, 16);
        assert_eq!(p.colors[5].r, 0xFF);
        ct[4] = 0x80;
        let (p, _) = Palette::from_color_table(&ct).unwrap();
        assert_eq!(p.colors[0].r, 0xFF);
    }
}
