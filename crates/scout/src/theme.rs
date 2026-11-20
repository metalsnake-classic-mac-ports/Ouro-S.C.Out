//! S.C.Out's look for the Ouro ports' shared screens (ouro-ui: the menu,
//! the level selection, the options and keys pages, the timeline): boxes
//! with a rim of the game's marble and the grey grid of its message boxes
//! inside (CODE 8 0x15ce's tiles), the chosen row in the cyan of its block
//! letters, text in the system font (the block letters are too large for
//! the shared layout), the title picture darkened behind.

use ouro_ui::{Ink, Rect, Role, Theme};
use scout_data::GameData;
use scout_data::graphics::TILE_SIZE;

use crate::render::{self, SCREEN_HEIGHT, SCREEN_WIDTH, Screen};
use crate::text::{Line, SystemFont};

/// The cyan of the block letters, white, grey.
const CYAN: [u8; 3] = [0x33, 0xBB, 0xEE];
const WHITE: [u8; 3] = [0xEE, 0xEE, 0xEE];
const GREY: [u8; 3] = [0x99, 0x99, 0x99];
const BLACK: [u8; 3] = [0, 0, 0];
/// The marble rim's width.
const RIM: i32 = 6;

pub struct Look<'a> {
    pub data: &'a GameData,
    pub font: &'a SystemFont,
}

fn size(role: Role) -> f32 {
    match role {
        Role::Heading => 24.0,
        Role::Row => 16.0,
        Role::Small | Role::Hint => 13.0,
    }
}

fn colour(ink: Ink) -> [u8; 3] {
    match ink {
        Ink::Normal => WHITE,
        Ink::Chosen => BLACK,
        Ink::Dim => GREY,
        Ink::Accent => CYAN,
    }
}

/// `r` cut to the screen.
fn on_screen(r: Rect) -> Rect {
    Rect::new(r.left.max(0), r.top.max(0), r.right.min(SCREEN_WIDTH as i32), r.bottom.min(SCREEN_HEIGHT as i32))
}

impl Look<'_> {
    /// The colour of a box tile's pixel (tile `k` of the 3 × 3).
    fn box_pixel(&self, k: usize, x: usize, y: usize) -> [u8; 3] {
        let Some(tile) = self.data.box_tiles.tile(k) else { return GREY };
        let c = self.data.game_palette.colors[tile[(y % TILE_SIZE) * TILE_SIZE + x % TILE_SIZE] as usize];
        [c.r, c.g, c.b]
    }
}

impl Theme for Look<'_> {
    type Surface = Screen;
    type Text = Line;

    fn width(&self, role: Role, text: &str) -> f32 {
        self.font.width(size(role), text)
    }

    fn backdrop(&self, s: &mut Screen, screen: Rect) {
        fill(s, screen, BLACK);
        s.picture(self.data, crate::menu::TITLE, render::PICTURE_AREA);
        for p in s.rgb.iter_mut() {
            *p = (u16::from(*p) * 2 / 5) as u8;
        }
    }

    /// The rim from the top edge's marble tile (1), the inside from the
    /// middle tile (4), its grid lined up with the box.
    fn panel(&self, s: &mut Screen, r: Rect) {
        let c = on_screen(r);
        for y in c.top..c.bottom {
            for x in c.left..c.right {
                let rim = (x - r.left).min(r.right - 1 - x).min(y - r.top).min(r.bottom - 1 - y);
                let p = if rim < RIM { self.box_pixel(1, x as usize, rim as usize) } else { self.box_pixel(4, (x - r.left) as usize, (y - r.top) as usize) };
                set(s, x, y, p);
            }
        }
    }

    fn highlight(&self, s: &mut Screen, r: Rect) {
        fill(s, r, CYAN);
    }

    fn button(&self, s: &mut Screen, r: Rect, _primary: bool, focused: bool) {
        fill(s, r, CYAN);
        if !focused {
            fill(s, r.inset(1), [0x44; 3]);
        }
    }

    fn rule(&self, s: &mut Screen, r: Rect) {
        fill(s, r, GREY);
    }

    fn fill(&self, s: &mut Screen, r: Rect, ink: Ink) {
        fill(s, r, colour(ink));
    }

    fn text(&self, x: f32, baseline: f32, role: Role, ink: Ink, text: &str) -> Line {
        let size = size(role);
        let mut line = Line::new(x, baseline - self.font.ascent(size), text).color(colour(ink));
        line.size = size;
        line
    }
}

fn set(s: &mut Screen, x: i32, y: i32, p: [u8; 3]) {
    let at = (y as usize * SCREEN_WIDTH + x as usize) * 3;
    s.rgb[at..at + 3].copy_from_slice(&p);
}

/// `r` in one colour.
pub fn fill(s: &mut Screen, r: Rect, p: [u8; 3]) {
    let c = on_screen(r);
    for y in c.top..c.bottom {
        for x in c.left..c.right {
            set(s, x, y, p);
        }
    }
}

/// `src` (RGB, `w` × `h`) into `r`, as large as fits without changing its
/// shape, centred; the rest black.
pub fn scaled_into(s: &mut Screen, r: Rect, src: &ouro_play::picture::Picture) {
    fill(s, r, BLACK);
    if src.width == 0 || src.height == 0 {
        return;
    }
    let scale = (r.width() as f32 / src.width as f32).min(r.height() as f32 / src.height as f32);
    let (w, h) = ((src.width as f32 * scale) as i32, (src.height as f32 * scale) as i32);
    let (left, top) = (r.left + (r.width() - w) / 2, r.top + (r.height() - h) / 2);
    for y in 0..h {
        let sy = ((y as f32 / scale) as usize).min(src.height - 1);
        for x in 0..w {
            let sx = ((x as f32 / scale) as usize).min(src.width - 1);
            let (dx, dy) = (left + x, top + y);
            if (0..SCREEN_WIDTH as i32).contains(&dx) && (0..SCREEN_HEIGHT as i32).contains(&dy) {
                let at = (sy * src.width + sx) * 3;
                set(s, dx, dy, [src.rgb[at], src.rgb[at + 1], src.rgb[at + 2]]);
            }
        }
    }
}
