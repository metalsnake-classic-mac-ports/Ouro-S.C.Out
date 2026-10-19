//! Projectiles (CODE 4 0x2392) and their tile handlers (tables `-0x3270`
//! "enter" and `-0x2aa0` "center" of the original).
//!
//! Projectiles move 2 pixels per frame. The original detects "entered a box"
//! and "at the box center" from exact pixel positions, so the step size is
//! part of the rules.

use super::{AROUND, index};
use crate::Level;
use crate::game::{BOX, Game};

pub const SLOTS: usize = 40;
/// Lifetime of a homing projectile in frames.
const HOMING_LIFE: i16 = 0xE10;
/// Hits a reinforced wall (tile 218) takes before it explodes.
const WALL_HITS: u16 = 20;
const EXPLOSION: u16 = 350;
/// Right / bottom edge of the level in pixels where projectiles stop.
const RIGHT_EDGE: i32 = 0x4FC;
const BOTTOM_EDGE: i32 = 0x31C;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Normal = 1,
    /// Bounces off walls.
    Ricochet = 2,
    /// Follows the player for a while.
    Homing = 3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Projectile {
    pub active: bool,
    /// Drawing state of the original: 0 off screen, 3 just drawn, 2 on
    /// screen, 1 to be erased. A slot is only reused once it is 0, so this
    /// matters for which slot a new projectile gets.
    pub draw: u8,
    /// Frames left (homing projectiles).
    pub life: i16,
    /// Position in level pixels.
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub kind: Kind,
}

impl Projectile {
    fn stop(&mut self) {
        self.active = false;
        if self.draw > 1 {
            self.draw = 1;
        }
    }
}

/// Tile values between which projectiles hit a wall.
fn is_wall_tile(tile: u16) -> bool {
    (40..=94).contains(&tile)
}

impl Game {
    /// A wall for projectiles: outside the level or tiles 40–94
    /// (CODE 9 0x1fe4).
    fn wall(&self, x: i32, y: i32) -> bool {
        index(x, y).is_none() || is_wall_tile(self.level.tile(x as usize, y as usize))
    }

    /// Take the first slot that is free and erased, as all spawn functions
    /// of the original do; fields not set by the caller keep old values.
    fn take_slot(&mut self, x: i32, y: i32, kind: Kind) -> Option<usize> {
        let slot = self
            .world
            .projectiles
            .iter()
            .position(|p| !p.active && p.draw == 0)?;
        self.world.projectile_count += 1;
        let p = &mut self.world.projectiles[slot];
        p.active = true;
        p.draw = 0;
        p.x = x;
        p.y = y;
        p.kind = kind;
        Some(slot)
    }

    /// Start a projectile at level pixel (x, y) (CODE 9 0x2132). Returns
    /// None when all slots are taken.
    pub(crate) fn spawn(&mut self, x: i32, y: i32, vx: i32, vy: i32) -> Option<usize> {
        let slot = self.take_slot(x, y, Kind::Normal)?;
        let p = &mut self.world.projectiles[slot];
        p.vx = vx;
        p.vy = vy;
        Some(slot)
    }

    /// A ricochet projectile from the box center in a random diagonal
    /// direction (CODE 9 0x21ba). The random number is only drawn when a
    /// slot is free.
    fn spawn_random_diagonal(&mut self, bx: i32, by: i32) {
        let Some(slot) = self.take_slot(bx * BOX + 16, by * BOX + 16, Kind::Ricochet) else {
            return;
        };
        let (vx, vy) = match self.world.random.next_positive() % 4 {
            0 => (2, 2),
            1 => (-2, 2),
            2 => (2, -2),
            _ => (-2, -2),
        };
        let p = &mut self.world.projectiles[slot];
        p.vx = vx;
        p.vy = vy;
    }

    /// A homing projectile from the box center (CODE 9 0x229e); its
    /// velocity is set when it moves.
    fn spawn_homing(&mut self, bx: i32, by: i32) {
        if let Some(slot) = self.take_slot(bx * BOX + 16, by * BOX + 16, Kind::Homing) {
            self.world.projectiles[slot].life = HOMING_LIFE;
        }
    }

    /// Move all projectiles one frame (CODE 4 0x2392).
    pub(crate) fn projectiles_frame(&mut self) {
        for i in 0..SLOTS {
            let p = &mut self.world.projectiles[i];
            if p.draw == 3 {
                p.draw = 2;
            }
            if !p.active {
                if p.draw == 1 {
                    self.world.projectile_count -= 1;
                    p.draw = 0;
                }
                continue;
            }
            let hit = if p.kind == Kind::Homing {
                self.move_homing(i)
            } else {
                match self.move_straight(i) {
                    Some(hit) => hit,
                    None => continue,
                }
            };
            self.projectile_common(i, hit);
        }
    }

    /// Move a homing projectile; returns its box and the tile there.
    fn move_homing(&mut self, i: usize) -> (i32, i32, u16) {
        let (px, py) = (
            self.player.x as i32 * BOX + 16,
            self.player.y as i32 * BOX + 16,
        );
        let p = &mut self.world.projectiles[i];
        p.vx = if p.x < px { 2 } else { -2 };
        p.vy = if p.y < py { 2 } else { -2 };
        p.x += p.vx;
        p.y += p.vy;
        let (mut bx, mut by) = (p.x / BOX, p.y / BOX);
        let mut tile = self.tile_at(bx, by);
        if is_wall_tile(tile) {
            // The original lets chance decide which step to take back first.
            let x_first = self.world.random.next_positive() % 2 != 0;
            let p = self.world.projectiles[i];
            if x_first {
                bx = (p.x - p.vx) / BOX;
            } else {
                by = (p.y - p.vy) / BOX;
            }
            tile = self.tile_at(bx, by);
            let both = is_wall_tile(tile);
            let p = &mut self.world.projectiles[i];
            if x_first || both {
                p.x -= p.vx;
            }
            if !x_first || both {
                p.y -= p.vy;
            }
        }
        (bx, by, tile)
    }

    /// Move a normal or ricochet projectile and run the tile handlers.
    /// Returns the box and tile for the common part, or None when the frame
    /// of this projectile is over.
    fn move_straight(&mut self, i: usize) -> Option<(i32, i32, u16)> {
        let p = &mut self.world.projectiles[i];
        p.x += p.vx;
        p.y += p.vy;
        let (x, y, vx, vy, kind) = (p.x, p.y, p.vx, p.vy, p.kind);
        let (bx, by) = (x / BOX, y / BOX);
        if index(bx, by).is_none() {
            p.stop();
            return None;
        }
        let tile = self.level.tile(bx as usize, by as usize);
        if tile == 0 {
            return Some((bx, by, tile));
        }
        // Just entered the box (at 2 pixels per frame).
        let entered = (vx > 0 && x % BOX == 0)
            || (vx < 0 && (x + 2) % BOX == 0)
            || (vy > 0 && y % BOX == 0)
            || (vy < 0 && (y + 2) % BOX == 0);
        if entered {
            if is_wall_tile(tile) {
                if kind == Kind::Ricochet {
                    self.ricochet(i, bx, by);
                } else {
                    self.world.projectiles[i].stop();
                }
                return None;
            }
            if (192..=214).contains(&tile) {
                self.set(bx, by, EXPLOSION, true);
                self.world.projectiles[i].stop();
                return None;
            }
            if self.projectile_enters(i, bx, by, tile) {
                return None;
            }
        }
        let at_center = ((x + 16) % BOX == 0 && (vy == 0 || kind == Kind::Ricochet))
            || ((y + 16) % BOX == 0 && vx == 0);
        if at_center && !self.free(bx, by) && self.projectile_at_center(i, bx, by, tile) {
            return None;
        }
        Some((bx, by, tile))
    }

    /// A ricochet projectile hit the wall box (bx, by): step back and turn
    /// depending on the walls around it, like the original (including its
    /// asymmetric choice of boxes).
    fn ricochet(&mut self, i: usize, bx: i32, by: i32) {
        let p = &mut self.world.projectiles[i];
        p.x -= p.vx;
        p.y -= p.vy;
        let (vx, vy) = (p.vx, p.vy);
        let (turn_x, turn_y) = if vx < 1 {
            if vy < 1 {
                (self.wall(bx, by + 1), !self.free(bx + 1, by))
            } else {
                (self.wall(bx, by - 1), self.wall(bx + 1, by))
            }
        } else if vy < 1 {
            (self.wall(bx, by + 1), self.wall(bx - 1, by))
        } else {
            (self.wall(bx, by - 1), self.wall(bx - 1, by))
        };
        let p = &mut self.world.projectiles[i];
        if turn_x {
            p.vx = -p.vx;
        }
        if turn_y {
            p.vy = -p.vy;
        }
        if !turn_x && !turn_y {
            p.vx = -p.vx;
            p.vy = -p.vy;
        }
    }

    /// The part of the projectile update after moving: drawing state, the
    /// edge of the level, the lifetime of homing projectiles and hitting
    /// the player.
    fn projectile_common(&mut self, i: usize, (bx, by, tile): (i32, i32, u16)) {
        let (left, top) = (self.view.0 as i32 * BOX, self.view.1 as i32 * BOX);
        let p = self.world.projectiles[i];
        let visible = left <= p.x && p.x <= left + 0x1BC && top <= p.y && p.y <= top + 0x13C;
        let draw = match p.draw {
            0 if visible => 3,
            2 if !visible => 1,
            1 if !visible => 0,
            other => other,
        };
        self.world.projectiles[i].draw = draw;
        let at_edge_x = p.x == 0 || p.x == RIGHT_EDGE;
        let at_edge_y = p.y == 0 || p.y == BOTTOM_EDGE;
        if (at_edge_x || at_edge_y) && p.active {
            if p.kind == Kind::Ricochet {
                if at_edge_x {
                    let turn = if p.vy < 1 {
                        self.wall(bx, by - 1)
                    } else {
                        self.wall(bx, by + 1)
                    };
                    let q = &mut self.world.projectiles[i];
                    q.vx = -q.vx;
                    if turn {
                        q.vy = -q.vy;
                    }
                }
                if at_edge_y {
                    let vx = self.world.projectiles[i].vx;
                    let turn = if vx < 1 {
                        self.wall(bx - 1, by)
                    } else {
                        self.wall(bx + 1, by)
                    };
                    let q = &mut self.world.projectiles[i];
                    q.vy = -q.vy;
                    if turn {
                        q.vx = -q.vx;
                    }
                }
            } else {
                self.world.projectiles[i].stop();
            }
        }
        let p = &mut self.world.projectiles[i];
        if p.kind == Kind::Homing {
            p.life -= 1;
            if p.life == 0 {
                p.stop();
            } else {
                p.active = true;
                if p.draw == 1 {
                    p.draw = 2;
                }
            }
        }
        if self.player_at(bx, by) && self.world.player_visible && self.world.dying == 0 {
            self.world.projectiles[i].stop();
            if !self.world.no_death && tile != 0x10E {
                self.world.player_hit = true;
            }
        }
    }

    /// The "enter" handler of a tile (table `-0x3270`). Returns whether the
    /// tile has one; then the frame of this projectile is over.
    fn projectile_enters(&mut self, i: usize, x: i32, y: i32, tile: u16) -> bool {
        let p = self.world.projectiles[i];
        match tile {
            // Reflectors (CODE 4 0x3f… ): straight back, or only one axis.
            24 | 25 => {
                let q = &mut self.world.projectiles[i];
                if q.vy == 0 {
                    q.vx = -q.vx;
                } else {
                    q.vy = -q.vy;
                }
            }
            28 | 219 => {
                if p.vy == 0 {
                    self.world.projectiles[i].vx = -p.vx;
                }
            }
            29 | 275 => {
                if p.vx == 0 {
                    self.world.projectiles[i].vy = -p.vy;
                }
            }
            // Direction objects: the player gets a command.
            20..=23 => {
                self.set(x, y, EXPLOSION, true);
                self.world.forced_command = [0, 3, 2, 1][(tile - 20) as usize];
            }
            // Splitter: four projectiles from the center.
            34 => {
                let (cx, cy) = (x * BOX + 16, y * BOX + 16);
                for (vx, vy) in [(-2, 0), (0, -2), (2, 0), (0, 2)] {
                    self.spawn(cx, cy, vx, vy);
                }
            }
            // Switching rails.
            144..=159 => {
                const RAIL: [u16; 16] = [
                    150, 151, 152, 153, 158, 159, 144, 145, 146, 147, 156, 157, 154, 155, 148, 149,
                ];
                self.set(x, y, RAIL[(tile - 144) as usize], true);
                self.world.projectiles[i].stop();
            }
            160 => self.replace_and_stop(i, x, y, 213),
            179 => self.world.projectiles[i].stop(),
            182 | 183 => self.replace_and_stop(i, x, y, 355),
            184 => self.replace_and_stop(i, x, y, 185),
            162..=165 => {
                self.world.switches[(tile - 162) as usize] = true;
                self.world.projectiles[i].stop();
                self.set(x, y, EXPLOSION, true);
            }
            // Chain reaction; the player survives it.
            33 => {
                self.set(x, y, 0, true);
                self.world.no_death = true;
                self.world.projectiles[i].stop();
                for (dx, dy) in AROUND {
                    self.explode(x + dx, y + dy);
                }
                self.explode(x, y);
            }
            32 => {
                self.spread(x, y, 208);
                self.world.projectiles[i].stop();
            }
            161 => {
                self.spread(x, y, 56);
                self.world.projectiles[i].stop();
            }
            166 => {
                self.computer(x, y, i);
            }
            // Reinforced wall: explodes after 20 hits (CODE 9 0x5586).
            218 => {
                self.hit_wall(x, y);
                self.world.projectiles[i].stop();
            }
            // Shooter downwards.
            222 => {
                if self.world.projectile_count != SLOTS as i32
                    && p.kind != Kind::Ricochet
                    && p.vy >= 0
                {
                    self.spawn(x * BOX + 16, y * BOX + 30, 0, 2);
                    self.world.projectiles[i].stop();
                }
            }
            227 => self.spawn_random_diagonal(x, y),
            228 => self.spawn_homing(x, y),
            // Missile box: explodes and leaves missiles in all free directions.
            265 => {
                self.set(x, y, EXPLOSION, true);
                for (dx, dy, missile) in [(-1, 0, 16), (0, -1, 17), (1, 0, 18), (0, 1, 19)] {
                    if self.free(x + dx, y + dy) {
                        self.set(x + dx, y + dy, missile, true);
                    }
                }
                self.world.projectiles[i].stop();
            }
            _ => return false,
        }
        true
    }

    fn replace_and_stop(&mut self, i: usize, x: i32, y: i32, tile: u16) {
        self.set(x, y, tile, true);
        self.world.projectiles[i].stop();
    }

    /// Computer terminal (CODE 4 0x3d5c): the program chips around it
    /// (0x110 / 0x111 / 0x112) decide which computer doors switch.
    fn computer(&mut self, x: i32, y: i32, i: usize) {
        let around: Vec<u16> = AROUND
            .iter()
            .map(|&(dx, dy)| self.tile_at(x + dx, y + dy))
            .collect();
        let count = |v: u16| around.iter().filter(|&&t| t == v).count();
        let (a, b, c) = (count(0x110), count(0x111), count(0x112));
        self.events.push(super::Event::Computer);
        if a == 0 && b == 0 && c == 0 {
            return;
        }
        let (open, close) = if b < a {
            (true, a <= c)
        } else if b < c {
            (true, true)
        } else {
            (false, true)
        };
        for n in 0..Level::CELLS {
            let (bx, by) = ((n % Level::WIDTH) as i32, (n / Level::WIDTH) as i32);
            match self.level.tile(bx as usize, by as usize) {
                0xE7 if open => self.set(bx, by, 91, true),
                0x56 if close => self.set(bx, by, 87, true),
                _ => {}
            }
        }
        self.world.projectiles[i].stop();
    }

    fn hit_wall(&mut self, x: i32, y: i32) {
        let Some(n) = index(x, y) else { return };
        self.world.wall_hits[n] += 1;
        if self.world.wall_hits[n] >= WALL_HITS {
            self.explode(x, y);
        }
    }

    /// The "center" handler of a tile (table `-0x2aa0`). Returns whether
    /// the tile has one; then the frame of this projectile is over.
    fn projectile_at_center(&mut self, i: usize, x: i32, y: i32, tile: u16) -> bool {
        let p = self.world.projectiles[i];
        match tile {
            // Enemies, also their second box.
            285 | 287 | 289 | 293 | 295 | 297 | 299 | 301 | 304 | 306 | 308 | 310 | 312 => {
                self.hit_pair(i, x, y, 1, 0)
            }
            294 | 296 | 298 | 300 | 302 | 305 | 307 | 309 | 311 | 313 => {
                self.hit_pair(i, x, y, -1, 0)
            }
            315 | 317 | 319 | 321 | 323 | 326 | 328 | 330 | 332 | 334 => {
                self.hit_pair(i, x, y, 0, 1)
            }
            316 | 318 | 320 | 322 | 324 | 327 | 329 | 331 | 333 | 335 => {
                self.hit_pair(i, x, y, 0, -1)
            }
            292 | 303 | 314 | 325 => self.replace_and_stop(i, x, y, EXPLOSION),
            // Mirrors.
            26 | 220 | 27 | 221 => {
                let anti = matches!(tile, 26 | 220);
                let q = &mut self.world.projectiles[i];
                if p.kind == Kind::Normal {
                    let sign = if anti { -1 } else { 1 };
                    if q.vx == 0 {
                        q.vx = sign * q.vy;
                        q.vy = 0;
                    } else {
                        q.vy = sign * q.vx;
                        q.vx = 0;
                    }
                } else {
                    let same_sign = (q.vx > 0 && q.vy > 0) || (q.vx < 0 && q.vy < 0);
                    let opposite = (q.vx > 0 && q.vy < 0) || (q.vx < 0 && q.vy > 0);
                    if (anti && same_sign) || (!anti && opposite) {
                        q.vx = -q.vx;
                        q.vy = -q.vy;
                    }
                }
            }
            // Cannons: a projectile passing the center fires them.
            223..=226 => {
                let (ox, oy, vx, vy) = match tile {
                    223 => (0, 16, -2, 0),
                    224 => (16, 0, 0, -2),
                    225 => (30, 16, 2, 0),
                    _ => (16, 30, 0, 2),
                };
                if self.spawn(x * BOX + ox, y * BOX + oy, vx, vy).is_some() {
                    self.events.push(super::Event::Cannon);
                }
            }
            _ => return false,
        }
        true
    }

    fn hit_pair(&mut self, i: usize, x: i32, y: i32, dx: i32, dy: i32) {
        self.set(x, y, EXPLOSION, true);
        self.set(x + dx, y + dy, EXPLOSION, true);
        self.world.projectiles[i].stop();
    }
}
