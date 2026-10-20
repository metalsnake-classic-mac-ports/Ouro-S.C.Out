//! Explosions (CODE 4 0x0 and the original's explosion handler table
//! `-0x22d0`, CODE 4 0x0186 …).

use super::AROUND;
use crate::game::Game;

/// Explosion animation the original puts on exploding boxes.
const EXPLOSION: u16 = 350;
/// The big explosion animation (the player's death, the destroyed target).
pub(crate) const BIG_EXPLOSION: u16 = 0x150;
/// Frames between the player's death and the end of the level (`-0x5396`).
pub(crate) const DEATH_FRAMES: u16 = 300;
/// Frames between destroying the target and the end of the level (`-0x3a46`).
pub(crate) const TARGET_FRAMES: u16 = 120;
/// A tile on which the player survives explosions.
const SAFE_TILE: u16 = 0x10E;

impl Game {
    /// An explosion on a box: the box's tile reacts, and a player standing
    /// there dies unless protected (CODE 4 0x0).
    pub(crate) fn explode(&mut self, x: i32, y: i32) {
        if super::index(x, y).is_none() {
            return;
        }
        let tile = self.level.tile(x as usize, y as usize);
        self.explosion_handler(x, y, tile);
        if !self.world.no_death && self.player_at(x, y) && tile != SAFE_TILE {
            self.set(x, y, BIG_EXPLOSION, true);
            self.world.dying = DEATH_FRAMES;
            self.world.player_visible = false;
        }
    }

    fn explosion_handler(&mut self, x: i32, y: i32, tile: u16) {
        match tile {
            // Things that turn into the explosion animation.
            0
            | 24..=29
            | 98..=102
            | 203
            | 218..=221
            | 275
            | 292
            | 303
            | 314
            | 325
            | 350..=353
            | 400
            | 415
            | 430
            | 445 => self.set(x, y, EXPLOSION, true),
            // Enemies between two boxes take the other box with them.
            285 | 287 | 289 | 293 | 295 | 297 | 299 | 301 | 304 | 306 | 308 | 310 | 312 | 401
            | 403 | 405 | 407 | 409 | 411 | 413 | 416 | 418 | 420 | 422 | 424 | 426 | 428 => {
                self.explode_pair(x, y, 1, 0)
            }
            294 | 296 | 298 | 300 | 302 | 305 | 307 | 309 | 311 | 313 | 402 | 404 | 406 | 408
            | 410 | 412 | 414 | 417 | 419 | 421 | 423 | 425 | 427 | 429 => {
                self.explode_pair(x, y, -1, 0)
            }
            315 | 317 | 319 | 321 | 323 | 326 | 328 | 330 | 332 | 334 | 431 | 433 | 435 | 437
            | 439 | 441 | 443 | 446 | 448 | 450 | 452 | 454 | 456 | 458 => {
                self.explode_pair(x, y, 0, 1)
            }
            316 | 318 | 320 | 322 | 324 | 327 | 329 | 331 | 333 | 335 | 432 | 434 | 436 | 438
            | 440 | 442 | 444 | 447 | 449 | 451 | 453 | 455 | 457 | 459 => {
                self.explode_pair(x, y, 0, -1)
            }
            // The player's death animation starts again.
            336..=342 => self.set(x, y, BIG_EXPLOSION, true),
            343 if !self.world.no_death => self.set(x, y, BIG_EXPLOSION, true),
            182 | 183 => self.set(x, y, 355, true),
            160 => self.set(x, y, 213, true),
            184 => self.set(x, y, 185, true),
            // Colored circles set the switch of their doors.
            162..=165 => {
                self.world.switches[(tile - 162) as usize] = true;
                self.set(x, y, EXPLOSION, true);
            }
            // The target: the level ends after a while.
            179 => {
                self.world.level_end = TARGET_FRAMES;
                self.set(x, y, BIG_EXPLOSION, true);
            }
            // Chain reaction; the player survives it.
            33 => {
                self.world.no_death = true;
                self.set(x, y, EXPLOSION, true);
                for (dx, dy) in AROUND {
                    self.explode(x + dx, y + dy);
                }
            }
            // Spreading into the free boxes around.
            32 => self.spread(x, y, 208),
            161 => self.spread(x, y, 56),
            _ => {}
        }
    }

    fn explode_pair(&mut self, x: i32, y: i32, dx: i32, dy: i32) {
        self.set(x, y, EXPLOSION, true);
        self.set(x + dx, y + dy, EXPLOSION, true);
    }

    /// The box and every free box around it (but not the player's) get
    /// `tile` (CODE 4 0x0186, 0x04da).
    pub(super) fn spread(&mut self, x: i32, y: i32, tile: u16) {
        self.set(x, y, tile, true);
        for (dx, dy) in AROUND {
            let (nx, ny) = (x + dx, y + dy);
            if self.free(nx, ny) && !self.player_at(nx, ny) {
                self.set(nx, ny, tile, true);
            }
        }
    }
}
