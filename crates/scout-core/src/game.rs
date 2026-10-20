//! The running game: the main loop of the original (CODE 9, 0x0).
//!
//! [`Game::frame`] is one pass of that loop: counters, keyboard, player,
//! world, projectiles and the countdowns of death and level end.
//! [`Game::tick`] wraps it for a frontend running at 60 frames per second:
//! the start animation and the extra frames the original spends drawing
//! while it scrolls or teleports.

use crate::Level;
use crate::world::{Event, World};

/// Size of a box in pixels.
pub const BOX: i32 = 32;
/// Pixels per frame the player moves (the original resets its speed
/// variable to 4 when the accelerator runs out).
pub const NORMAL_SPEED: i32 = 4;
/// Frames of the animation on the start box when a level begins
/// (CODE 5 ≈ 0x2ee0): tiles 0xab … 0xb2, eight frames each.
pub const INTRO_FRAMES: usize = 64;
const INTRO_FIRST_TILE: u16 = 0xAB;
/// The closed hatch the start box shows after the animation.
pub const START_HATCH: u16 = 0x10B;
/// Lives at the start of a game (CODE 2).
pub const LIVES: i16 = 5;
/// Frames between the player's death and the end of the level.
const DEATH_FRAMES: u16 = 300;
/// The big explosion the player turns into.
const BIG_EXPLOSION: u16 = 0x150;

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

    pub(crate) fn from_index(i: i32) -> Option<Dir> {
        Dir::ALL.get(usize::try_from(i).ok()?).copied()
    }
}

/// The key held in a frame. The original only looks at the first pressed
/// key, so there is at most one per frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Key {
    #[default]
    None,
    Move(Dir),
    /// Take, drop, swap or teleport (command 7; acts when pressed).
    Use,
    /// Fire the carried missile (command 4; acts when pressed).
    Fire,
    /// Key `5` on the keypad: blow yourself up.
    SelfDestruct,
}

/// How a level ended (`-0x6d0c`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// Lost a life (2).
    Died,
    /// The target was destroyed (3).
    Won,
    /// Lost a life by self destruction (0xfe).
    SelfDestructed,
    /// Lost the last life (0xff).
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    /// Box position in the level (`-0x579a`, `-0x5796`).
    pub x: usize,
    pub y: usize,
    /// Direction of the hull (`-0x57aa`).
    pub dir: Dir,
    /// Direction of the turret, the sprite variant (`-0x5786`).
    pub turret: Dir,
    /// The turret stays put: a missile is loaded (`-0x5788`).
    pub turret_locked: bool,
    /// Pixels left of the current move, 0 = standing (`-0x57a6`).
    pub remaining: i32,
    /// Pixels per frame (`-0x57ae`).
    pub speed: i32,
    /// The command of the frame (`-0x5790`): 0–3 move, 4 fire, 7 use,
    /// 8 self destruct, −1 none. Kept while moving.
    pub command: i32,
    /// A cannon under the player has fired since the last move (`-0x5398`).
    pub cannon_fired: bool,
    /// Frames left of the accelerator (`-0x538e`).
    pub accelerator: u16,
    /// Frames left of the magnifier (`-0x538c`).
    pub magnifier: u16,
    /// Self destruction was used in this level (`-0x6cfc`).
    pub self_destructed: bool,
    /// Lives left (`-0x6dac`).
    pub lives: i16,
    use_held: bool,
    fire_held: bool,
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

    /// The box the player is moving to (`-0x53c0`, `-0x53bc`).
    pub fn target(&self) -> (usize, usize) {
        if self.remaining == 0 {
            return (self.x, self.y);
        }
        let (dx, dy) = self.dir.delta();
        ((self.x as i32 + dx) as usize, (self.y as i32 + dy) as usize)
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub level: Level,
    pub player: Player,
    /// Top left box of the 14 × 10 view (`-0x57c6`, `-0x57c2`).
    pub view: (usize, usize),
    pub world: World,
    /// Events of the last frames for the frontend; it drains them.
    pub events: Vec<Event>,
    /// How the level ended; no more frames run then.
    pub exit: Option<Exit>,
    /// All projectiles stop at the end of the frame (`-0x6d00`).
    pub(crate) clear_shots: bool,
    /// Next step of the start animation while it runs.
    intro: Option<usize>,
    /// Frames the original spends drawing after the last frame (scrolling
    /// one box per frame, teleporting); [`Game::tick`] waits them out.
    pub(crate) stall: Stall,
}

/// The view moving to its new position for the frontend: the logic has
/// moved it at once, the original showed one box per frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stall {
    pub frames: usize,
    /// View before the move and boxes per frame towards the new one.
    pub scroll: Option<((usize, usize), Dir)>,
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
                turret_locked: false,
                remaining: 0,
                speed: NORMAL_SPEED,
                command: -1,
                cannon_fired: false,
                accelerator: 0,
                magnifier: 0,
                self_destructed: false,
                lives: LIVES,
                use_held: false,
                fire_held: false,
            },
            view,
            world: World::default(),
            events: Vec::new(),
            exit: None,
            clear_shots: false,
            intro: None,
            stall: Stall::default(),
        }
    }

    /// Begin the level like the original: the start box (0x108 after
    /// loading) shows the opening animation for [`INTRO_FRAMES`] frames,
    /// then the closed hatch. Nothing else runs meanwhile.
    pub fn start_intro(&mut self) {
        let (x, y) = (self.player.x as i32, self.player.y as i32);
        self.set(x, y, INTRO_FIRST_TILE, false);
        self.world.player_visible = false;
        self.intro = Some(1);
    }

    /// The state right after the start animation.
    pub fn skip_intro(&mut self) {
        let (x, y) = (self.player.x as i32, self.player.y as i32);
        self.set(x, y, START_HATCH, false);
        self.world.player_visible = true;
        self.intro = None;
    }

    pub fn in_intro(&self) -> bool {
        self.intro.is_some()
    }

    /// The view to draw: lags behind [`Game::view`] while the original
    /// would still be scrolling.
    pub fn shown_view(&self) -> (usize, usize) {
        match self.stall.scroll {
            Some((from, dir)) if self.stall.frames > 0 => {
                let (dx, dy) = dir.delta();
                let (fx, fy) = (from.0 as i32, from.1 as i32);
                let moved = |from: i32, to: usize, d: i32| {
                    if d == 0 {
                        to as i32
                    } else {
                        let steps = (to as i32 - from).abs() - self.stall.frames as i32;
                        from + d * steps.max(0)
                    }
                };
                (
                    moved(fx, self.view.0, dx) as usize,
                    moved(fy, self.view.1, dy) as usize,
                )
            }
            _ => self.view,
        }
    }

    /// Advance one frame of real time (1/60 s).
    pub fn tick(&mut self, key: Key) {
        if let Some(step) = self.intro {
            if step < INTRO_FRAMES {
                let (x, y) = (self.player.x as i32, self.player.y as i32);
                self.set(x, y, INTRO_FIRST_TILE + (step / 8) as u16, false);
                self.intro = Some(step + 1);
            } else {
                self.skip_intro();
            }
            return;
        }
        if self.stall.frames > 0 {
            self.stall.frames -= 1;
            if self.stall.frames == 0 {
                self.stall.scroll = None;
            }
            return;
        }
        self.frame(key);
    }

    /// One pass of the main loop (CODE 9 0x28 … 0x248).
    pub fn frame(&mut self, key: Key) {
        if self.exit.is_some() {
            return;
        }
        self.world.frame = (self.world.frame + 1) % crate::world::FRAMES_PER_PASS;
        if self.world.frame == 0 {
            self.world.pulse = (self.world.pulse + 1) % crate::world::PULSE_PERIOD;
        }
        if self.player.accelerator != 0 {
            self.player.accelerator -= 1;
            if self.player.accelerator == 0 {
                self.player.speed = NORMAL_SPEED;
            }
        }
        if self.player.remaining == 0 {
            if self.world.level_end == 0 {
                self.read_key(key);
            } else {
                self.player.command = -1;
            }
        }
        if self.player.magnifier == 0 {
            if self.world.dying == 0 {
                self.player_frame();
            } else {
                self.world.dying -= 1;
            }
            self.world_frame();
            self.projectiles_frame();
            if self.clear_shots {
                for p in self.world.projectiles.iter_mut() {
                    p.active = false;
                    if p.draw > 1 {
                        p.draw = 1;
                    }
                }
                self.clear_shots = false;
            }
        } else {
            self.player.magnifier -= 1;
            if self.player.magnifier == 0 {
                self.teleport(None);
                self.player.command = -1;
            } else {
                self.magnifier_frame();
            }
        }
        if self.world.level_end > 1 {
            self.world.level_end -= 1;
        }
        if self.world.level_end == 1 {
            if self.world.dying == 0 {
                self.exit = Some(Exit::Won);
            } else {
                self.world.dying = 1;
            }
        }
        if self.world.dying == 1 && self.world.level_end == 0 {
            self.player.lives -= 1;
            self.exit = Some(if self.player.lives == 0 {
                Exit::GameOver
            } else if self.player.self_destructed {
                Exit::SelfDestructed
            } else {
                Exit::Died
            });
        }
        if self.player.command == 8 && !self.player.self_destructed && self.player.magnifier == 0 {
            self.player.self_destructed = true;
            if self.world.dying == 0 {
                self.world.dying = DEATH_FRAMES;
                self.world.player_visible = false;
                let (x, y) = (self.player.x as i32, self.player.y as i32);
                self.set(x, y, BIG_EXPLOSION, false);
            }
        }
    }

    /// The keyboard routine (CODE 6 0x7a8): use and fire act once per
    /// press.
    fn read_key(&mut self, key: Key) {
        let p = &mut self.player;
        p.command = match key {
            Key::Move(dir) => dir as i32,
            Key::SelfDestruct => 8,
            _ => -1,
        };
        if key == Key::Use {
            if !p.use_held {
                p.command = 7;
            }
            p.use_held = true;
        } else {
            p.use_held = false;
        }
        if key == Key::Fire {
            if !p.fire_held {
                p.command = 4;
            }
            p.fire_held = true;
        } else {
            p.fire_held = false;
        }
    }

    /// Only the world and the projectiles, as the reference recordings of
    /// the world automaton were made (`re/tools/emu_world.py`).
    #[doc(hidden)]
    pub fn world_only_frame(&mut self) {
        self.world.frame = (self.world.frame + 1) % crate::world::FRAMES_PER_PASS;
        if self.world.frame == 0 {
            self.world.pulse = (self.world.pulse + 1) % crate::world::PULSE_PERIOD;
        }
        self.world_frame();
        self.projectiles_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::test_level;

    #[test]
    fn the_start_box_opens_and_closes() {
        let mut game = Game::new(test_level(&[], (5, 5)));
        assert_eq!(game.level.tile(5, 5), Level::START_TILE);
        game.start_intro();
        let mut shown = vec![game.level.tile(5, 5)];
        while game.in_intro() {
            assert!(!game.world.player_visible);
            game.tick(Key::Move(Dir::Right));
            shown.push(game.level.tile(5, 5));
        }
        assert_eq!(shown.len(), INTRO_FRAMES + 1);
        assert_eq!(shown[..8], [0xAB; 8]);
        assert_eq!(shown[63], 0xB2);
        assert_eq!(shown[64], START_HATCH);
        assert!(game.world.player_visible);
        assert_eq!((game.player.x, game.player.y), (5, 5));
    }

    /// A wall tile in the translated numbering (index 48: wall).
    const WALL: u16 = 48;

    fn run(game: &mut Game, key: Key, frames: usize) {
        for _ in 0..frames {
            game.frame(key);
        }
    }

    #[test]
    fn a_move_takes_eight_frames_and_starts_at_once() {
        let mut game = Game::new(test_level(&[], (5, 5)));
        assert_eq!(game.player.sprite(), 15, "starts facing down");
        game.frame(Key::Move(Dir::Right));
        assert_eq!(game.player.offset(), (4, 0));
        assert_eq!(game.player.sprite(), 4 * 2 + 2);
        run(&mut game, Key::None, 6);
        assert_eq!((game.player.x, game.player.offset()), (5, (28, 0)));
        game.frame(Key::None);
        assert_eq!((game.player.x, game.player.y), (6, 5));
        assert_eq!(game.player.offset(), (0, 0));
    }

    #[test]
    fn keys_are_ignored_while_moving() {
        let mut game = Game::new(test_level(&[], (5, 5)));
        game.frame(Key::Move(Dir::Right));
        run(&mut game, Key::Move(Dir::Down), 7);
        assert_eq!((game.player.x, game.player.y), (6, 5));
    }

    #[test]
    fn walls_block_and_the_player_does_not_turn() {
        let mut game = Game::new(test_level(&[(6, 5, WALL)], (5, 5)));
        game.frame(Key::Move(Dir::Right));
        assert_eq!(game.player.remaining, 0);
        assert_eq!(game.player.dir, Dir::Down);
    }

    #[test]
    fn one_way_doors_block_one_direction() {
        // 0xF5 cannot be entered moving right, but moving left is fine.
        let mut game = Game::new(test_level(&[(6, 5, 0xF5), (4, 5, 0xF5)], (5, 5)));
        game.frame(Key::Move(Dir::Right));
        assert_eq!(game.player.remaining, 0);
        game.frame(Key::Move(Dir::Left));
        assert!(game.player.remaining > 0);
    }

    #[test]
    fn the_view_scrolls_within_the_frame_of_arrival() {
        // Start view (0, 0); arriving at view column 12 scrolls by 6.
        let mut game = Game::new(test_level(&[], (10, 5)));
        assert_eq!(game.view, (0, 0));
        run(&mut game, Key::Move(Dir::Right), 16);
        assert_eq!(game.player.x, 12);
        assert_eq!(game.view, (6, 0));
        // The frontend shows the player at the new box first, then the
        // scroll over the following frames.
        assert_eq!(game.shown_view(), (0, 0));
        game.tick(Key::None);
        assert_eq!(game.shown_view(), (0, 0));
        game.tick(Key::None);
        assert_eq!(game.shown_view(), (1, 0));
    }

    #[test]
    fn teleporting_centers_the_view_on_the_target() {
        // Teleporter 0x101 at the start, its target 0x105 far down right.
        let mut game = Game::new(test_level(&[(5, 5, 0x101), (27, 17, 0x105)], (5, 5)));
        assert_eq!(game.view, (0, 0));
        game.set(5, 5, 0x101, false);
        game.frame(Key::Use);
        assert_eq!((game.player.x, game.player.y), (27, 17));
        // Target 7 / 5 boxes from the view's corner, clamped to the level.
        assert_eq!(game.view, (20, 12));
        // 3 boxes each way (the order of the steps cannot be observed).
        let mut game = Game::new(test_level(&[(7, 5, 0x101), (17, 13, 0x105)], (7, 5)));
        game.set(7, 5, 0x101, false);
        game.view = (7, 5);
        game.frame(Key::Use);
        assert_eq!(game.view, (10, 8));
        assert_eq!(game.stall.scroll, None);
    }

    #[test]
    fn use_takes_and_drops_objects() {
        // 25 is an object on the floor.
        let mut game = Game::new(test_level(&[(6, 5, 25)], (5, 5)));
        run(&mut game, Key::Move(Dir::Right), 8);
        game.frame(Key::Use);
        assert_eq!((game.world.carried, game.level.tile(6, 5)), (25, 0));
        game.frame(Key::Use);
        assert_eq!(game.world.carried, 25, "acts once per press");
        game.frame(Key::None);
        game.frame(Key::Use);
        assert_eq!((game.world.carried, game.level.tile(6, 5)), (0, 25));
    }
}
