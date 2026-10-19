//! The world automaton: the per-tile behavior of the original.
//!
//! The original (CODE 9 0x23fa) visits 96 of the 960 boxes per frame, so
//! every box once every 10 frames, and calls the tick handler of the box's
//! tile. Handlers replace tiles with `set` (CODE 9 0x2046), which can mark
//! the target box to be skipped once, so something that moves into a box
//! visited later in the same pass does not move twice.
//!
//! The handlers were taken from the original's handler table (see
//! `re/NOTES.md`); comments name the tile values and code addresses.

mod animations;
mod explosion;

use crate::Level;
use crate::game::Game;

/// Boxes the original visits per frame (`-0x6d04`, from its initialized data).
pub const BOXES_PER_FRAME: usize = 96;
/// Frames for one pass over the whole level.
pub const FRAMES_PER_PASS: usize = Level::CELLS / BOXES_PER_FRAME;
/// The pulse counter (`-0x5390`) advances once per pass and wraps at 30;
/// batteries fire when it is 0.
pub const PULSE_PERIOD: u16 = 30;
/// Floor the original puts under things that vanish.
const PLAIN_FLOOR: u16 = 13;

/// Something the game reports to the frontend (sounds and the like).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// The player got picked up by a rail or conveyor pulse.
    PickedUp,
}

/// State of the world automaton.
#[derive(Clone, Debug)]
pub struct World {
    /// Frame within the pass (`-0x6d08`).
    pub frame: usize,
    /// Pass counter for batteries (`-0x5390`).
    pub pulse: u16,
    skip: [bool; Level::CELLS],
    /// Switch flags (`-0x53b8` … `-0x53b4`) that open colored doors.
    pub switches: [bool; 5],
    /// The object the player carries (`-0x578c`), 0 = none.
    pub carried: u16,
    /// Set when a rail or conveyor pulse reaches the player (`-0x539e`).
    pub picked_up: bool,
    /// Explosions do not kill the player (`-0x5392`), set by a chain reaction.
    pub no_death: bool,
    /// Frames left after the player died (`-0x5396`), 0 = alive.
    pub dying: u16,
    /// Frames left until the level ends after the target was destroyed
    /// (`-0x3a46`), 0 = not destroyed.
    pub level_end: u16,
}

impl Default for World {
    fn default() -> Self {
        Self {
            frame: 0,
            pulse: 0,
            skip: [false; Level::CELLS],
            switches: [false; 5],
            carried: 0,
            picked_up: false,
            no_death: false,
            dying: 0,
            level_end: 0,
        }
    }
}

impl World {
    pub fn skip(&self, x: usize, y: usize) -> bool {
        self.skip[y * Level::WIDTH + x]
    }
}

impl Game {
    /// One frame of the world: advance the counters like the main loop of
    /// the original (CODE 9 0x0) and run the automaton on the next slice
    /// (CODE 9 0x23fa).
    pub(crate) fn world_frame(&mut self) {
        self.world.frame = (self.world.frame + 1) % FRAMES_PER_PASS;
        if self.world.frame == 0 {
            self.world.pulse = (self.world.pulse + 1) % PULSE_PERIOD;
        }
        let start = self.world.frame * BOXES_PER_FRAME;
        for i in start..start + BOXES_PER_FRAME {
            if self.world.skip[i] {
                self.world.skip[i] = false;
                continue;
            }
            let (x, y) = ((i % Level::WIDTH) as i32, (i / Level::WIDTH) as i32);
            let tile = self.level.tile(x as usize, y as usize);
            self.tick_box(x, y, tile);
        }
    }

    /// Replace a tile and set the box's skip flag (CODE 9 0x2046).
    fn set(&mut self, x: i32, y: i32, tile: u16, skip: bool) {
        if let Some(i) = index(x, y) {
            self.level.set_tile(x as usize, y as usize, tile);
            self.world.skip[i] = skip;
        }
    }

    /// Replace the floor and set the box's skip flag (CODE 9 0x20e4).
    fn set_floor(&mut self, x: i32, y: i32, floor: u16, skip: bool) {
        if let Some(i) = index(x, y) {
            self.level.set_floor(x as usize, y as usize, floor);
            self.world.skip[i] = skip;
        }
    }

    /// The tile of a box, 0 outside the level.
    fn tile_at(&self, x: i32, y: i32) -> u16 {
        index(x, y).map_or(0, |_| self.level.tile(x as usize, y as usize))
    }

    /// Whether a box is empty: inside the level and tile 0 (CODE 9 0x1f94).
    fn free(&self, x: i32, y: i32) -> bool {
        index(x, y).is_some() && self.level.tile(x as usize, y as usize) == 0
    }

    fn player_at(&self, x: i32, y: i32) -> bool {
        self.player.x as i32 == x && self.player.y as i32 == y
    }

    /// The thing on the box disappears, plain floor stays.
    fn vanish(&mut self, x: i32, y: i32) {
        self.set(x, y, 0, false);
        self.set_floor(x, y, PLAIN_FLOOR, false);
    }

    fn explode_around(&mut self, x: i32, y: i32) {
        for (dx, dy) in AROUND {
            self.explode(x + dx, y + dy);
        }
    }

    fn tick_box(&mut self, x: i32, y: i32, tile: u16) {
        if let Some(next) = animations::next_tile(tile) {
            self.set(x, y, next, false);
            return;
        }
        let (px, py) = (self.player.x as i32, self.player.y as i32);
        match tile {
            // Colored doors opening when their switch flag is set
            // (CODE 9 0x24c8 … 0x26c0); 61 opens when the flag is clear.
            61 if !self.world.switches[4] => self.set(x, y, 62, false),
            66 if self.world.switches[0] => self.set(x, y, 67, false),
            71 if self.world.switches[1] => self.set(x, y, 72, false),
            76 if self.world.switches[2] => self.set(x, y, 77, false),
            81 if self.world.switches[3] => self.set(x, y, 82, false),
            // Tiles reacting to whether the player carries something
            // (0x28be, 0x3d3e).
            103 if self.world.carried == 0 => self.set(x, y, 95, false),
            232 if self.world.carried != 0 => self.set(x, y, 99, false),
            // Doors opening when the player stands next to them on one side
            // (0x28dc … 0x2ba6).
            104 if px == x && py + 1 == y => self.set(x, y, 105, false),
            113 if px == x && py - 1 == y => self.set(x, y, 114, false),
            122 if px == x + 1 && py == y => self.set(x, y, 123, false),
            131 if px == x - 1 && py == y => self.set(x, y, 132, false),
            // Rail and conveyor pulses moving on (0x2c94 … 0x377c,
            // 0x3f48 … 0x4242).
            141 | 157 | 159 | 252 => self.pulse(x, y, tile, Pulse::LEFT),
            143 | 149 | 155 | 254 => self.pulse(x, y, tile, Pulse::RIGHT),
            145 | 147 | 170 | 250 => self.pulse(x, y, tile, Pulse::DOWN),
            151 | 153 | 168 | 248 => self.pulse(x, y, tile, Pulse::UP),
            // Battery: every 30 passes it powers the rail or conveyor below
            // (0x387c).
            181 if self.world.pulse == 0 => {
                let below = self.tile_at(x, y + 1);
                let rail = (0x8C..=0x9F).contains(&below) && below.is_multiple_of(2);
                let other = ((0xA7..=0xAA).contains(&below) || (0xF7..=0x103).contains(&below))
                    && below % 2 == 1;
                if rail || other {
                    self.set(x, y + 1, below + 1, false);
                }
            }
            // Spreads into all free boxes around it, except the player's
            // (0x3a9e); boxes after it in the pass are skipped once.
            217 => {
                for (i, (dx, dy)) in AROUND.into_iter().enumerate() {
                    let (nx, ny) = (x + dx, y + dy);
                    if self.free(nx, ny) && !self.player_at(nx, ny) {
                        self.set(nx, ny, 213, i >= 3);
                    }
                }
                self.set(x, y, 160, false);
            }
            // Things that vanish at the end of an animation.
            235 | 238 | 301 | 313 | 323 | 335 | 343 | 353 | 413 | 429 | 443 | 459 => {
                self.vanish(x, y)
            }
            // Open one-way doors close again once the player has left
            // (0x3e68 … 0x3f10).
            243 if !(px == x && (py + 1 == y || py == y)) => self.set(x, y, 109, false),
            244 if !(px == x && (py - 1 == y || py == y)) => self.set(x, y, 118, false),
            245 if !(py == y && (px - 1 == x || px == x)) => self.set(x, y, 127, false),
            246 if !(py == y && (px + 1 == x || px == x)) => self.set(x, y, 136, false),
            // Enemy turning clockwise (0x43d4, 0x4572, 0x4710, 0x48ae):
            // it tries to go on, else turns, else turns again.
            292 => self.enemy(x, y, &[ENEMY_UP, ENEMY_RIGHT], 314),
            303 => self.enemy(x, y, &[ENEMY_DOWN, ENEMY_LEFT], 325),
            314 => self.enemy(x, y, &[ENEMY_RIGHT, ENEMY_DOWN], 303),
            325 => self.enemy(x, y, &[ENEMY_LEFT, ENEMY_UP], 292),
            // Bombs exploding around themselves (0x4b88, 0x4c78).
            354 => {
                self.set(x, y, 351, false);
                self.explode_around(x, y);
            }
            358 => {
                self.vanish(x, y);
                self.explode_around(x, y);
            }
            // Enemies going back and forth (0x4d2e, 0x4ee8, 0x50a2, 0x525c).
            400 => self.enemy(x, y, &[STEP_RIGHT_400], 415),
            415 => self.enemy(x, y, &[STEP_LEFT_415], 400),
            430 => self.enemy(x, y, &[STEP_DOWN_430], 445),
            445 => self.enemy(x, y, &[STEP_UP_445], 430),
            _ => {}
        }
    }

    /// A pulse on a rail or conveyor: the box falls back to its unpowered
    /// tile; if the player is on it, they are picked up, otherwise the
    /// matching neighbor gets powered.
    fn pulse(&mut self, x: i32, y: i32, tile: u16, pulse: Pulse) {
        self.set(x, y, tile - 1, false);
        if self.player_at(x, y) {
            if !self.world.picked_up {
                self.events.push(Event::PickedUp);
            }
            self.world.picked_up = true;
            return;
        }
        let (nx, ny) = (x + pulse.dx, y + pulse.dy);
        let next = self.tile_at(nx, ny);
        if let Some(&(_, powered, skip)) = pulse.next.iter().find(|(from, _, _)| *from == next) {
            self.set(nx, ny, powered, skip);
        }
    }

    /// An enemy: try the moves in order; the first one with a free target
    /// box happens. If none is possible the enemy turns into `blocked`.
    fn enemy(&mut self, x: i32, y: i32, moves: &[EnemyMove], blocked: u16) {
        for m in moves {
            if self.free(x + m.dx, y + m.dy) {
                for &(dx, dy, tile, skip) in m.sets {
                    self.set(x + dx, y + dy, tile, skip);
                }
                return;
            }
        }
        self.set(x, y, blocked, false);
    }
}

fn index(x: i32, y: i32) -> Option<usize> {
    ((0..Level::WIDTH as i32).contains(&x) && (0..Level::HEIGHT as i32).contains(&y))
        .then(|| y as usize * Level::WIDTH + x as usize)
}

/// The eight neighbors in the order the original visits them.
const AROUND: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
];

/// Where a pulse goes and how it powers the next box:
/// (unpowered tile, powered tile, skip).
struct Pulse {
    dx: i32,
    dy: i32,
    next: &'static [(u16, u16, bool)],
}

impl Pulse {
    const LEFT: Pulse = Pulse {
        dx: -1,
        dy: 0,
        next: &[
            (0x9A, 155, false),
            (0x92, 147, false),
            (0x8C, 141, false),
            (0xFB, 252, false),
        ],
    };
    const RIGHT: Pulse = Pulse {
        dx: 1,
        dy: 0,
        next: &[
            (0x96, 151, true),
            (0x8E, 143, true),
            (0x90, 145, true),
            (0xFD, 254, true),
        ],
    };
    const DOWN: Pulse = Pulse {
        dx: 0,
        dy: 1,
        next: &[
            (0xA9, 170, true),
            (0x9E, 159, true),
            (0x94, 149, true),
            (0xF9, 250, true),
        ],
    };
    const UP: Pulse = Pulse {
        dx: 0,
        dy: -1,
        next: &[
            (0xA7, 168, false),
            (0x9A, 155, true),
            (0x9C, 157, true),
            (0xF7, 248, false),
        ],
    };
}

/// One possible enemy move: the box that must be free and the tiles to set
/// (dx, dy, tile, skip), in the original's order.
struct EnemyMove {
    dx: i32,
    dy: i32,
    sets: &'static [(i32, i32, u16, bool)],
}

const ENEMY_UP: EnemyMove = EnemyMove {
    dx: 0,
    dy: -1,
    sets: &[(0, -1, 326, false), (0, 0, 327, false)],
};
const ENEMY_RIGHT: EnemyMove = EnemyMove {
    dx: 1,
    dy: 0,
    sets: &[(0, 0, 293, false), (1, 0, 294, true)],
};
const ENEMY_DOWN: EnemyMove = EnemyMove {
    dx: 0,
    dy: 1,
    sets: &[(0, 1, 316, true), (0, 0, 315, false)],
};
const ENEMY_LEFT: EnemyMove = EnemyMove {
    dx: -1,
    dy: 0,
    sets: &[(0, 0, 305, false), (-1, 0, 304, false)],
};
const STEP_RIGHT_400: EnemyMove = EnemyMove {
    dx: 1,
    dy: 0,
    sets: &[(0, 0, 401, false), (1, 0, 402, true)],
};
const STEP_LEFT_415: EnemyMove = EnemyMove {
    dx: -1,
    dy: 0,
    sets: &[(-1, 0, 416, false), (0, 0, 417, false)],
};
const STEP_DOWN_430: EnemyMove = EnemyMove {
    dx: 0,
    dy: 1,
    sets: &[(0, 0, 431, false), (0, 1, 432, true)],
};
const STEP_UP_445: EnemyMove = EnemyMove {
    dx: 0,
    dy: -1,
    sets: &[(0, 0, 447, false), (0, -1, 446, false)],
};
