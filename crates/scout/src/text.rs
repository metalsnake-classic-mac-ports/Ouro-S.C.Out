//! Text in the system's standard font, for what the block letters of the
//! game cannot show (key names, hints). It is drawn at the resolution of the
//! window on top of the 640 × 480 picture.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};

/// A line of text at a position of the 640 × 480 screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub x: f32,
    /// Top of the line.
    pub y: f32,
    /// Height in screen pixels.
    pub size: f32,
    pub color: [u8; 3],
    pub text: String,
    pub align: Align,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
}

impl Line {
    pub fn new(x: f32, y: f32, text: impl Into<String>) -> Self {
        Self {
            x,
            y,
            size: 15.0,
            color: [0xDD, 0xDD, 0xDD],
            text: text.into(),
            align: Align::Left,
        }
    }

    pub fn centered(mut self) -> Self {
        self.align = Align::Center;
        self
    }

    pub fn color(mut self, color: [u8; 3]) -> Self {
        self.color = color;
        self
    }
}

/// Families tried first, by system; then any sans-serif font.
#[cfg(target_os = "macos")]
const FAMILIES: &[&str] = &["SF Pro", "SF Pro Text", "Helvetica Neue", "Helvetica"];
#[cfg(target_os = "windows")]
const FAMILIES: &[&str] = &["Segoe UI", "Tahoma", "Arial"];
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const FAMILIES: &[&str] = &[
    "Noto Sans",
    "DejaVu Sans",
    "Liberation Sans",
    "Cantarell",
    "Ubuntu",
];

/// The macOS system font is hidden from the font list.
#[cfg(target_os = "macos")]
const SYSTEM_FONT_FILE: Option<&str> = Some("/System/Library/Fonts/SFNS.ttf");
#[cfg(not(target_os = "macos"))]
const SYSTEM_FONT_FILE: Option<&str> = None;

pub struct SystemFont {
    font: Option<FontVec>,
}

impl SystemFont {
    /// Find the font; without one, text is simply not shown.
    pub fn load() -> Self {
        let font = SYSTEM_FONT_FILE
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|data| FontVec::try_from_vec(data).ok())
            .or_else(find_font);
        if font.is_none() {
            eprintln!("no system font found: hints and key names are not shown");
        }
        Self { font }
    }

    /// Draw the lines into an RGBA image of `width` × `height` pixels that
    /// covers the 640 × 480 screen.
    pub fn draw(&self, lines: &[Line], width: usize, height: usize) -> Vec<u8> {
        let mut rgba = vec![0u8; width * height * 4];
        let Some(font) = &self.font else {
            return rgba;
        };
        let scale = width as f32 / 640.0;
        for line in lines {
            let px = line.size * scale;
            let scaled = font.as_scaled(PxScale::from(px));
            let mut glyphs = Vec::new();
            let mut caret = 0.0;
            let mut last = None;
            for c in line.text.chars() {
                let id = font.glyph_id(c);
                if let Some(prev) = last {
                    caret += scaled.kern(prev, id);
                }
                glyphs.push(id.with_scale_and_position(px, ab_glyph::point(caret, 0.0)));
                caret += scaled.h_advance(id);
                last = Some(id);
            }
            let left = match line.align {
                Align::Left => line.x * scale,
                Align::Center => line.x * scale - caret / 2.0,
            };
            let baseline = line.y * scale + scaled.ascent();
            for glyph in glyphs {
                let Some(outline) = font.outline_glyph(glyph) else {
                    continue;
                };
                let bounds = outline.px_bounds();
                outline.draw(|gx, gy, coverage| {
                    let x = (left + bounds.min.x) as i32 + gx as i32;
                    let y = (baseline + bounds.min.y) as i32 + gy as i32;
                    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                        return;
                    }
                    let at = (y as usize * width + x as usize) * 4;
                    let a = (coverage.clamp(0.0, 1.0) * 255.0) as u8;
                    if a > rgba[at + 3] {
                        rgba[at..at + 4].copy_from_slice(&[
                            line.color[0],
                            line.color[1],
                            line.color[2],
                            a,
                        ]);
                    }
                });
            }
        }
        rgba
    }
}

fn find_font() -> Option<FontVec> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let mut families: Vec<fontdb::Family> =
        FAMILIES.iter().map(|&f| fontdb::Family::Name(f)).collect();
    families.push(fontdb::Family::SansSerif);
    let id = db.query(&fontdb::Query {
        families: &families,
        ..Default::default()
    })?;
    db.with_face_data(id, |data, index| {
        FontVec::try_from_vec_and_index(data.to_vec(), index).ok()
    })
    .flatten()
}
