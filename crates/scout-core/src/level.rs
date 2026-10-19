/// One level: a grid of 40 × 24 boxes.
///
/// Every box has a tile value and a floor value, numbered like in the level
/// files of the original game (the numbering of the MS-DOS version, which the
/// C++ port uses too). The Mac graphics use other numbers; the frontend
/// translates them when drawing.
#[derive(Clone, PartialEq, Eq)]
pub struct Level {
    tiles: [u16; Self::CELLS],
    floor: [u8; Self::CELLS],
}

impl Level {
    pub const WIDTH: usize = 40;
    pub const HEIGHT: usize = 24;
    pub const CELLS: usize = Self::WIDTH * Self::HEIGHT;
    /// Size of the part of the level visible on screen, in boxes.
    pub const VIEW_WIDTH: usize = 14;
    pub const VIEW_HEIGHT: usize = 10;
    /// Tile values that mark the start box. The original level loader
    /// (CODE 5, 0x48c) clears these boxes and takes the last one as start.
    const START_VALUES: [u16; 15] = [1, 2, 3, 10, 11, 12, 13, 20, 21, 22, 23, 30, 31, 32, 33];

    pub fn new(tiles: [u16; Self::CELLS], floor: [u8; Self::CELLS]) -> Self {
        Self { tiles, floor }
    }

    pub fn tile(&self, x: usize, y: usize) -> u16 {
        self.tiles[Self::index(x, y)]
    }

    pub fn floor(&self, x: usize, y: usize) -> u8 {
        self.floor[Self::index(x, y)]
    }

    /// The start box: the last box (row by row) with a start value.
    pub fn start(&self) -> Option<(usize, usize)> {
        let i = self
            .tiles
            .iter()
            .rposition(|t| Self::START_VALUES.contains(t))?;
        Some((i % Self::WIDTH, i / Self::WIDTH))
    }

    /// Whether this box is the start marker, which the game does not draw.
    pub fn is_start_marker(&self, x: usize, y: usize) -> bool {
        Self::START_VALUES.contains(&self.tile(x, y))
    }

    /// The top left box of the view when the level starts, like the original
    /// computes it: the view snaps to 14 × 10 sections, but moves so the
    /// player is at least two boxes away from the edge of the view.
    pub fn start_view(&self) -> (usize, usize) {
        let (x, y) = self.start().unwrap_or((0, 0));
        let (x, y) = (x as i32, y as i32);
        let mut vx = x / 14 * 14;
        if x == vx {
            vx -= 2;
        }
        if x - vx == 1 {
            vx -= 1;
        }
        if x - vx == 12 {
            vx += 1;
        }
        if x - vx == 13 {
            vx += 2;
        }
        let mut vy = y / 10 * 10;
        if y - vy == 1 {
            vy -= 1;
        }
        if y - vy == 8 {
            vy += 1;
        }
        if y == vy {
            vy -= 2;
        }
        if y - vy == 9 {
            vy += 2;
        }
        let max_x = (Self::WIDTH - Self::VIEW_WIDTH) as i32;
        let max_y = (Self::HEIGHT - Self::VIEW_HEIGHT) as i32;
        (vx.clamp(0, max_x) as usize, vy.clamp(0, max_y) as usize)
    }

    fn index(x: usize, y: usize) -> usize {
        assert!(
            x < Self::WIDTH && y < Self::HEIGHT,
            "box ({x}, {y}) is outside the level"
        );
        y * Self::WIDTH + x
    }
}

impl std::fmt::Debug for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Level").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_are_stored_row_by_row() {
        let mut tiles = [0; Level::CELLS];
        let mut floor = [0; Level::CELLS];
        tiles[Level::WIDTH + 2] = 300;
        floor[Level::WIDTH + 2] = 7;
        let level = Level::new(tiles, floor);
        assert_eq!(level.tile(2, 1), 300);
        assert_eq!(level.floor(2, 1), 7);
        assert_eq!(level.tile(1, 2), 0);
    }

    fn level_with_start(x: usize, y: usize) -> Level {
        let mut tiles = [0; Level::CELLS];
        tiles[y * Level::WIDTH + x] = 11;
        Level::new(tiles, [0; Level::CELLS])
    }

    #[test]
    fn start_is_the_last_start_value() {
        let mut tiles = [0; Level::CELLS];
        tiles[5] = 11;
        tiles[3 * Level::WIDTH + 7] = 22;
        let level = Level::new(tiles, [0; Level::CELLS]);
        assert_eq!(level.start(), Some((7, 3)));
        assert!(level.is_start_marker(5, 0));
        assert_eq!(
            Level::new([0; Level::CELLS], [0; Level::CELLS]).start(),
            None
        );
    }

    #[test]
    fn start_view_keeps_the_player_away_from_the_edge() {
        // Level 1 of the original: start (2, 2), view at the top left.
        assert_eq!(level_with_start(2, 2).start_view(), (0, 0));
        // On the left edge of a section the view moves 2 boxes left ...
        assert_eq!(level_with_start(14, 12).start_view(), (12, 10));
        // ... one box from the edge it moves 1 box.
        assert_eq!(level_with_start(15, 11).start_view(), (13, 9));
        // Near the right / bottom edge of a section it moves right / down.
        assert_eq!(level_with_start(26, 18).start_view(), (15, 11));
        assert_eq!(level_with_start(27, 19).start_view(), (16, 12));
        // Clamped to the level.
        assert_eq!(level_with_start(39, 23).start_view(), (26, 14));
        assert_eq!(level_with_start(0, 0).start_view(), (0, 0));
    }

    #[test]
    #[should_panic(expected = "outside the level")]
    fn reading_outside_panics() {
        Level::new([0; Level::CELLS], [0; Level::CELLS]).tile(Level::WIDTH, 0);
    }
}
