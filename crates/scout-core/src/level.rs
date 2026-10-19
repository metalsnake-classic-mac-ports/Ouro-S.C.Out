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

    pub fn new(tiles: [u16; Self::CELLS], floor: [u8; Self::CELLS]) -> Self {
        Self { tiles, floor }
    }

    pub fn tile(&self, x: usize, y: usize) -> u16 {
        self.tiles[Self::index(x, y)]
    }

    pub fn floor(&self, x: usize, y: usize) -> u8 {
        self.floor[Self::index(x, y)]
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

    #[test]
    #[should_panic(expected = "outside the level")]
    fn reading_outside_panics() {
        Level::new([0; Level::CELLS], [0; Level::CELLS]).tile(Level::WIDTH, 0);
    }
}
