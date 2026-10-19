//! The running game: player, view and the per-frame update.
//!
//! Follows the main loop of the original (CODE 9, 0x0) and its player
//! update (CODE 9, 0xb70). One call of [`Game::tick`] is one frame of the
//! original.

use crate::Level;
use crate::world::{Event, World};

/// Size of a box in pixels.
pub const BOX: i32 = 32;
/// Pixels per frame the player moves (the original resets its speed
/// variable to 4 when the accelerator runs out).
pub const NORMAL_SPEED: i32 = 4;

/// A direction, numbered like in the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Left = 0,
    Up = 1,
    Right = 2,
    Down = 3,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Left, Dir::Up, Dir::Right, Dir::Down];

    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Left => (-1, 0),
            Dir::Up => (0, -1),
            Dir::Right => (1, 0),
            Dir::Down => (0, 1),
        }
    }

    /// One-way door tile that cannot be entered when moving this way.
    fn blocking_one_way(self) -> u16 {
        match self {
            Dir::Left => 0xF6,
            Dir::Up => 0xF3,
            Dir::Right => 0xF5,
            Dir::Down => 0xF4,
        }
    }
}

/// The input of one frame. The original only looks at the first pressed
/// key, so there is at most one command per frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Command {
    #[default]
    None,
    Move(Dir),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    /// Box position in the level.
    pub x: usize,
    pub y: usize,
    /// Direction of the hull.
    pub dir: Dir,
    /// Direction of the turret (the sprite variant).
    pub turret: Dir,
    /// Pixels left of the current move (0 = standing).
    pub remaining: i32,
    pub speed: i32,
}

impl Player {
    /// The object graphic of the player: 4 × turret + hull direction.
    pub fn sprite(&self) -> usize {
        4 * self.turret as usize + self.dir as usize
    }

    /// Pixel offset from the box position while moving.
    pub fn offset(&self) -> (i32, i32) {
        let (dx, dy) = self.dir.delta();
        let done = if self.remaining == 0 {
            0
        } else {
            BOX - self.remaining
        };
        (dx * done, dy * done)
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub level: Level,
    pub player: Player,
    /// Top left box of the 14 × 10 view.
    pub view: (usize, usize),
    /// A running scroll of the view: direction and boxes left.
    scroll: Option<(Dir, usize)>,
    pub world: World,
    /// Events of the last frames for the frontend; it drains them.
    pub events: Vec<Event>,
}

impl Game {
    pub fn new(level: Level) -> Self {
        let (x, y) = level.start().unwrap_or((1, 1));
        let view = level.start_view();
        Self {
            level,
            player: Player {
                x,
                y,
                dir: Dir::Down,
                turret: Dir::Down,
                remaining: 0,
                speed: NORMAL_SPEED,
            },
            view,
            scroll: None,
            world: World::default(),
            events: Vec::new(),
        }
    }

    pub fn is_scrolling(&self) -> bool {
        self.scroll.is_some()
    }

    /// Advance one frame.
    pub fn tick(&mut self, command: Command) {
        // The original scrolls one box per frame and runs no player update
        // meanwhile (CODE 9, 0x5b2 and friends).
        if let Some((dir, left)) = self.scroll {
            let (dx, dy) = dir.delta();
            self.view.0 = (self.view.0 as i32 + dx) as usize;
            self.view.1 = (self.view.1 as i32 + dy) as usize;
            self.scroll = (left > 1).then_some((dir, left - 1));
            return;
        }
        if self.player.remaining == 0
            && let Command::Move(dir) = command
        {
            self.try_start_move(dir);
        }
        if self.player.remaining > 0 {
            self.player.remaining = (self.player.remaining - self.player.speed).max(0);
            if self.player.remaining == 0 {
                self.arrive();
            }
        }
        self.world_frame();
        self.projectiles_frame();
    }

    /// Whether a box in view-relative coordinates can be entered
    /// (CODE 9, 0x1f12): the view's edge blocks; floor and objects (below 40),
    /// tiles 184–188 and tiles from 219 up are free, other tiles block.
    fn passable(&self, rel_x: i32, rel_y: i32) -> bool {
        if !(0..Level::VIEW_WIDTH as i32).contains(&rel_x)
            || !(0..Level::VIEW_HEIGHT as i32).contains(&rel_y)
        {
            return false;
        }
        let x = self.view.0 + rel_x as usize;
        let y = self.view.1 + rel_y as usize;
        let tile = self.level.tile(x, y);
        tile < Level::FIRST_SOLID_TILE || (0xB8..=0xBC).contains(&tile) || tile >= 0xDB
    }

    fn try_start_move(&mut self, dir: Dir) {
        let (dx, dy) = dir.delta();
        let rel_x = self.player.x as i32 - self.view.0 as i32 + dx;
        let rel_y = self.player.y as i32 - self.view.1 as i32 + dy;
        if !self.passable(rel_x, rel_y) {
            return;
        }
        let target = self.level.tile(
            (self.player.x as i32 + dx) as usize,
            (self.player.y as i32 + dy) as usize,
        );
        if target == dir.blocking_one_way() {
            return;
        }
        self.player.dir = dir;
        self.player.turret = dir;
        self.player.remaining = BOX;
    }

    /// The move to the next box is complete: update the box position and
    /// start scrolling when the player is near the edge of the view.
    fn arrive(&mut self) {
        let (dx, dy) = self.player.dir.delta();
        self.player.x = (self.player.x as i32 + dx) as usize;
        self.player.y = (self.player.y as i32 + dy) as usize;
        let (vx, vy) = (self.view.0 as i32, self.view.1 as i32);
        let rel_x = self.player.x as i32 - vx;
        let rel_y = self.player.y as i32 - vy;
        let boxes = match self.player.dir {
            Dir::Left if rel_x == 1 && vx != 0 => vx.min(6),
            Dir::Up if rel_y == 1 && vy != 0 => vy.min(4),
            Dir::Right if rel_x == 12 && vx != 26 => {
                if rel_x + vx + 7 > 39 {
                    38 - (rel_x + vx)
                } else {
                    6
                }
            }
            Dir::Down if rel_y == 8 && vy != 15 => {
                if rel_y + vy + 5 > 23 {
                    22 - (rel_y + vy)
                } else {
                    4
                }
            }
            _ => 0,
        };
        if boxes > 0 {
            self.scroll = Some((self.player.dir, boxes as usize));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::test_level;

    /// A wall tile in the translated numbering (index 48: wall).
    const WALL: u16 = 48;

    fn run(game: &mut Game, command: Command, frames: usize) {
        for _ in 0..frames {
            game.tick(command);
        }
    }

    #[test]
    fn a_move_takes_eight_frames() {
        let mut game = Game::new(test_level(&[], (5, 5)));
        assert_eq!(
            game.player.sprite(),
            15,
            "starts facing down like the original"
        );
        game.tick(Command::Move(Dir::Right));
        assert_eq!(game.player.offset(), (4, 0));
        assert_eq!(game.player.sprite(), 4 * 2 + 2);
        run(&mut game, Command::None, 6);
        assert_eq!((game.player.x, game.player.offset()), (5, (28, 0)));
        game.tick(Command::None);
        assert_eq!((game.player.x, game.player.y), (6, 5));
        assert_eq!(game.player.offset(), (0, 0));
    }

    #[test]
    fn keys_are_ignored_while_moving() {
        let mut game = Game::new(test_level(&[], (5, 5)));
        game.tick(Command::Move(Dir::Right));
        run(&mut game, Command::Move(Dir::Down), 7);
        assert_eq!((game.player.x, game.player.y), (6, 5));
    }

    #[test]
    fn walls_block_and_the_player_does_not_turn() {
        let mut game = Game::new(test_level(&[(6, 5, WALL)], (5, 5)));
        game.tick(Command::Move(Dir::Right));
        assert_eq!(game.player.remaining, 0);
        assert_eq!(game.player.dir, Dir::Down);
    }

    #[test]
    fn passable_tiles() {
        for (tile, free) in [
            (0, true),
            (39, true),
            (40, false),
            (183, false),
            (184, true),
            (188, true),
            (189, false),
            (218, false),
            (219, true),
            (300, true),
        ] {
            let mut game = Game::new(test_level(&[(6, 5, tile)], (5, 5)));
            game.tick(Command::Move(Dir::Right));
            assert_eq!(game.player.remaining > 0, free, "tile {tile}");
        }
    }

    #[test]
    fn one_way_doors_block_one_direction() {
        // 0xF5 cannot be entered moving right, but moving left is fine.
        let mut game = Game::new(test_level(&[(6, 5, 0xF5), (4, 5, 0xF5)], (5, 5)));
        game.tick(Command::Move(Dir::Right));
        assert_eq!(game.player.remaining, 0);
        game.tick(Command::Move(Dir::Left));
        assert!(game.player.remaining > 0);
    }

    #[test]
    fn the_view_scrolls_one_box_per_frame_near_the_edge() {
        // Start view (0, 0); walking right to view column 12 scrolls by 6.
        let mut game = Game::new(test_level(&[], (10, 5)));
        assert_eq!(game.view, (0, 0));
        run(&mut game, Command::Move(Dir::Right), 8);
        assert_eq!(game.player.x, 11);
        assert!(!game.is_scrolling());
        run(&mut game, Command::Move(Dir::Right), 8);
        assert_eq!(game.player.x, 12);
        assert!(game.is_scrolling());
        run(&mut game, Command::Move(Dir::Right), 6);
        assert_eq!(game.view, (6, 0));
        assert!(!game.is_scrolling());
        assert_eq!(game.player.remaining, 0, "no moves while scrolling");
    }

    #[test]
    fn scrolling_stops_at_the_level_edge() {
        let mut game = Game::new(test_level(&[], (35, 5)));
        assert_eq!(game.view.0, 26);
        // Walking left to view column 1 scrolls by 6.
        let (_, _) = game.view;
        run(&mut game, Command::Move(Dir::Left), 8 * 8);
        assert_eq!(game.player.x, 27);
        assert!(game.is_scrolling());
        run(&mut game, Command::None, 6);
        assert_eq!(game.view.0, 20);
    }
}
