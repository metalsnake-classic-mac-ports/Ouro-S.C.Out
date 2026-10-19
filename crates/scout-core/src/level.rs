use crate::translate;

/// One level: a grid of 40 × 24 boxes.
///
/// The level files use the numbering of the MS-DOS version (the C++ port
/// uses it too). Like the Mac version, [`Level::from_file`] translates the
/// values when loading; all game logic works on the translated values:
/// a tile value below [`Level::FIRST_SOLID_TILE`] is an object (or nothing)
/// on the floor, a value from 40 up is a tile that covers the whole box.
/// Both are indices into the Mac graphics.
#[derive(Clone, PartialEq, Eq)]
pub struct Level {
    tiles: [u16; Self::CELLS],
    floor: [u16; Self::CELLS],
    file_tiles: [u16; Self::CELLS],
    file_floor: [u8; Self::CELLS],
    start: Option<(usize, usize)>,
}

impl Level {
    pub const WIDTH: usize = 40;
    pub const HEIGHT: usize = 24;
    pub const CELLS: usize = Self::WIDTH * Self::HEIGHT;
    /// Size of the part of the level visible on screen, in boxes.
    pub const VIEW_WIDTH: usize = 14;
    pub const VIEW_HEIGHT: usize = 10;
    /// Tile values from here on cover the whole box, below are objects.
    pub const FIRST_SOLID_TILE: u16 = 40;
    /// The tile the original puts on the start box after loading.
    pub const START_TILE: u16 = 0x108;
    /// File values that mark the start box. The original level loader
    /// (CODE 5, 0x48c) clears these boxes and takes the last one as start.
    const START_VALUES: [u16; 15] = [1, 2, 3, 10, 11, 12, 13, 20, 21, 22, 23, 30, 31, 32, 33];

    /// Build a level from the values of the level file, like the original
    /// loader does: translate tiles and floor, find the start box, clear the
    /// start markers and put [`Level::START_TILE`] on the start box.
    pub fn from_file(file_tiles: [u16; Self::CELLS], file_floor: [u8; Self::CELLS]) -> Self {
        let mut tiles = [0; Self::CELLS];
        let mut floor = [0; Self::CELLS];
        let mut start = None;
        for i in 0..Self::CELLS {
            if Self::START_VALUES.contains(&file_tiles[i]) {
                start = Some((i % Self::WIDTH, i / Self::WIDTH));
            } else {
                tiles[i] = translate::tile(file_tiles[i]);
            }
            floor[i] = translate::floor(file_floor[i]);
        }
        if let Some((x, y)) = start {
            tiles[y * Self::WIDTH + x] = Self::START_TILE;
        }
        Self {
            tiles,
            floor,
            file_tiles,
            file_floor,
            start,
        }
    }

    /// Build a level from already translated values (tests, editors).
    /// The start box keeps its tile; the file values are set to 0.
    pub fn from_translated(
        tiles: [u16; Self::CELLS],
        floor: [u16; Self::CELLS],
        start: Option<(usize, usize)>,
    ) -> Self {
        Self {
            tiles,
            floor,
            file_tiles: [0; Self::CELLS],
            file_floor: [0; Self::CELLS],
            start,
        }
    }

    /// The (translated) tile value of a box.
    pub fn tile(&self, x: usize, y: usize) -> u16 {
        self.tiles[Self::index(x, y)]
    }

    pub fn set_tile(&mut self, x: usize, y: usize, value: u16) {
        self.tiles[Self::index(x, y)] = value;
    }

    /// The (translated) floor tile of a box.
    pub fn floor(&self, x: usize, y: usize) -> u16 {
        self.floor[Self::index(x, y)]
    }

    pub fn set_floor(&mut self, x: usize, y: usize, value: u16) {
        self.floor[Self::index(x, y)] = value;
    }

    /// The tile value as stored in the level file.
    pub fn file_tile(&self, x: usize, y: usize) -> u16 {
        self.file_tiles[Self::index(x, y)]
    }

    /// The floor value as stored in the level file.
    pub fn file_floor(&self, x: usize, y: usize) -> u8 {
        self.file_floor[Self::index(x, y)]
    }

    /// The start box: the last box (row by row) with a start value.
    pub fn start(&self) -> Option<(usize, usize)> {
        self.start
    }

    /// The top left box of the view when the level starts, like the original
    /// computes it: the view snaps to 14 × 10 sections, but moves so the
    /// player is at least two boxes away from the edge of the view.
    pub fn start_view(&self) -> (usize, usize) {
        let (x, y) = self.start.unwrap_or((0, 0));
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
        f.debug_struct("Level")
            .field("start", &self.start)
            .finish_non_exhaustive()
    }
}

/// Build a level from translated values for tests: everything empty floor,
/// the given tiles, and the start box.
#[cfg(test)]
pub(crate) fn test_level(tiles: &[(usize, usize, u16)], start: (usize, usize)) -> Level {
    let mut file_tiles = [0; Level::CELLS];
    file_tiles[start.1 * Level::WIDTH + start.0] = 11;
    let mut level = Level::from_file(file_tiles, [0; Level::CELLS]);
    for &(x, y, v) in tiles {
        level.set_tile(x, y, v);
    }
    level
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level_with_start(x: usize, y: usize) -> Level {
        test_level(&[], (x, y))
    }

    #[test]
    fn loading_translates_and_marks_the_start() {
        let mut tiles = [0; Level::CELLS];
        let mut floor = [0; Level::CELLS];
        tiles[Level::WIDTH + 2] = 26; // a wall in the file numbering
        floor[Level::WIDTH + 2] = 41;
        tiles[5] = 11;
        tiles[3 * Level::WIDTH + 7] = 22;
        let level = Level::from_file(tiles, floor);
        assert_eq!(level.tile(2, 1), translate::tile(26));
        assert_eq!(level.floor(2, 1), translate::floor(41));
        assert_eq!(level.file_tile(2, 1), 26);
        assert_eq!(level.start(), Some((7, 3)));
        assert_eq!(level.tile(7, 3), Level::START_TILE);
        assert_eq!(level.tile(5, 0), 0, "earlier start markers are cleared");
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
        Level::from_file([0; Level::CELLS], [0; Level::CELLS]).tile(Level::WIDTH, 0);
    }
}
