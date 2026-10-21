//! The player update of the original (CODE 9 0xb70) and what it calls:
//! tiles acting on the player, taking and dropping, firing, moving,
//! scrolling the view (CODE 9 0x3c6 …), teleporters and the magnifier
//! (CODE 4 0x2054, 0x1fde).

use crate::Level;
use crate::game::{BOX, Dir, Game, NORMAL_SPEED};
use crate::sound::id as sound;

/// Frames of the accelerator (tile 0x10d) at double speed.
const ACCELERATOR_FRAMES: u16 = 600;
/// Frames of the magnifier (tile 0x10c).
const MAGNIFIER_FRAMES: u16 = 300;
const PLAIN_FLOOR: u16 = 13;
/// From this tile value on, tiles are enemies that explode on contact.
const FIRST_ENEMY: i32 = 0x124;

/// Rail tiles that steer the player while on a rail (CODE 9 0xefa …).
const RAILS: [(i32, i32); 16] = [
    (140, 0),
    (142, 2),
    (144, 3),
    (146, 3),
    (148, 2),
    (150, 1),
    (152, 1),
    (154, 2),
    (156, 0),
    (158, 0),
    (167, 1),
    (169, 3),
    (247, 1),
    (249, 3),
    (251, 0),
    (253, 2),
];

impl Game {
    /// The box of the player relative to the view.
    fn rel(&self) -> (i32, i32) {
        (
            self.player.x as i32 - self.view.0 as i32,
            self.player.y as i32 - self.view.1 as i32,
        )
    }

    fn here(&self) -> (i32, i32) {
        (self.player.x as i32, self.player.y as i32)
    }

    /// The player's pixel position relative to the view (`-0x57be`,
    /// `-0x57ba`).
    pub fn player_pixel(&self) -> (i32, i32) {
        let (rx, ry) = self.rel();
        let (ox, oy) = self.player.offset();
        (rx * BOX + ox, ry * BOX + oy)
    }

    /// Whether a box in view-relative coordinates can be entered
    /// (CODE 9, 0x1f12): the view's edge blocks; floor and objects (below
    /// 40), tiles 184–188 and tiles from 219 up are free, others block;
    /// with the cheat NOWALL everything in view is free.
    fn passable(&self, rel_x: i32, rel_y: i32) -> bool {
        if !(0..Level::VIEW_WIDTH as i32).contains(&rel_x)
            || !(0..Level::VIEW_HEIGHT as i32).contains(&rel_y)
        {
            return false;
        }
        if self.no_walls {
            return true;
        }
        let x = self.view.0 + rel_x as usize;
        let y = self.view.1 + rel_y as usize;
        let tile = self.level.tile(x, y);
        tile < Level::FIRST_SOLID_TILE || (0xB8..=0xBC).contains(&tile) || tile >= 0xDB
    }

    /// The player update (CODE 9 0xb70).
    pub(crate) fn player_frame(&mut self) {
        self.world.player_visible = true;
        if self.player.remaining == 0 && !self.player_at_rest() {
            return;
        }
        self.player_move();
    }

    /// Everything that happens while the player stands on a box; false
    /// when the frame is over for the player.
    fn player_at_rest(&mut self) -> bool {
        let (x, y) = self.here();
        if self.world.player_hit {
            self.set(x, y, 0x150, true);
            self.world.dying = 300;
            self.world.player_visible = false;
            return false;
        }
        self.mines();
        let tile = self.level.tile(x as usize, y as usize) as i32;
        if tile >= FIRST_ENEMY {
            self.explode(x, y);
        }
        let rail = (0x8C..=0xAA).contains(&tile);
        if self.world.picked_up && !rail && !(0xF7..=0xFE).contains(&tile) {
            self.world.picked_up = false;
        }
        if rail && self.player.command <= 3 {
            self.player.command = -1;
        }
        let mut rail_dir = -1;
        let (rx, ry) = self.rel();
        let (vx, vy) = (self.view.0 as i32 * BOX, self.view.1 as i32 * BOX);
        match tile {
            232 if self.world.carried != 0 => self.explode(x, y),
            99..=103 => self.explode(x, y),
            // Conveyors keep the player going in its direction.
            239..=242 if self.player.command != 4 => {
                let (dx, dy) = self.player.dir.delta();
                if self.passable(rx + dx, ry + dy) {
                    self.player.command = self.player.dir as i32;
                }
            }
            // Cannons under the player fire once per visit.
            223..=226 if !self.player.cannon_fired => {
                let (ox, oy, vx_, vy_) = match tile {
                    223 => (0, 16, -2, 0),
                    224 => (16, 0, 0, -2),
                    225 => (30, 16, 2, 0),
                    _ => (16, 30, 0, 2),
                };
                let fired = self.spawn(vx + rx * BOX + ox, vy + ry * BOX + oy, vx_, vy_);
                self.player.cannon_fired = true;
                if fired.is_some() {
                    self.request_sound(sound::CANNON);
                }
            }
            // A hatch: closes and saves the state to restart from.
            266 => {
                self.set(x, y, 0x10B, true);
                self.checkpoint();
            }
            // Accelerator.
            269 => {
                self.player.accelerator = ACCELERATOR_FRAMES;
                self.request_sound(sound::ACCELERATOR);
                self.player.speed = 2 * NORMAL_SPEED;
                self.set(x, y, 0, true);
                self.set_floor(x, y, PLAIN_FLOOR, true);
            }
            // Magnifier: the view can be moved around for a while.
            268 => {
                self.player.magnifier = MAGNIFIER_FRAMES;
                self.set(x, y, 0, true);
                self.set_floor(x, y, PLAIN_FLOOR, true);
                self.projectiles_frame();
                self.world.player_visible = false;
                self.request_sound(sound::MAGNIFIER);
                self.stall.frames += 1;
                return false;
            }
            270 if self.sounds.playing != Some(sound::SAFE) => self.request_sound(sound::SAFE),
            276 => self.clear_shots = true,
            _ => {
                if self.world.picked_up
                    && let Some(&(_, dir)) = RAILS.iter().find(|(t, _)| *t == tile)
                {
                    rail_dir = dir;
                }
            }
        }
        if self.world.forced_command != -1 {
            self.player.command = self.world.forced_command as i32;
            self.world.forced_command = -1;
        }
        match self.player.command {
            0..=3 => self.try_start_move(Dir::from_index(self.player.command).unwrap(), tile),
            7 => self.use_key(tile),
            4 => self.fire(),
            _ => {}
        }
        if !(0..=3).contains(&self.player.command)
            && let Some(dir) = Dir::from_index(rail_dir)
        {
            self.start_move(dir);
        }
        true
    }

    /// Mines around the player (CODE 4 0x1c50): 0xb6 and 0xb7 explode
    /// together with the player's box, 0xb8 changes to 0xb9.
    fn mines(&mut self) {
        let (x, y) = self.here();
        for (dx, dy) in [
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
        ] {
            let (nx, ny) = (x + dx, y + dy);
            match self.tile_at(nx, ny) {
                0xB8 => self.set(nx, ny, 0xB9, true),
                0xB6 | 0xB7 => {
                    self.explode(nx, ny);
                    self.explode(x, y);
                }
                _ => {}
            }
        }
    }

    /// A move key (CODE 9 0x1016 …): the box must be passable and not a
    /// one-way door against the direction; enemies there explode
    /// together with the player's box.
    fn try_start_move(&mut self, dir: Dir, here: i32) {
        let (rx, ry) = self.rel();
        let (dx, dy) = dir.delta();
        if !self.passable(rx + dx, ry + dy) {
            return;
        }
        let (x, y) = self.here();
        let target = self.tile_at(x + dx, y + dy) as i32;
        let one_way = match dir {
            Dir::Left => 0xF6,
            Dir::Up => 0xF3,
            Dir::Right => 0xF5,
            Dir::Down => 0xF4,
        };
        if target == one_way {
            return;
        }
        if target >= FIRST_ENEMY {
            self.explode(x + dx, y + dy);
            self.explode(x, y);
        }
        self.start_move(dir);
        self.player.cannon_fired = false;
        if self.sounds.on && !self.sounds.busy && self.sounds.request.is_none() {
            let conveyor = (0xEF..=0xF2).contains(&here);
            self.request_sound(if conveyor {
                sound::CONVEYOR
            } else {
                sound::MOVE
            });
        }
    }

    fn start_move(&mut self, dir: Dir) {
        let p = &mut self.player;
        p.dir = dir;
        if !p.turret_locked {
            p.turret = dir;
        }
        p.remaining = BOX;
    }

    /// Command 7 (CODE 9 0x1332): take, swap or drop what is on the box,
    /// or use a teleporter.
    fn use_key(&mut self, tile: i32) {
        let (x, y) = self.here();
        if self.world.carried == 0 {
            if (0x101..=0x104).contains(&tile) {
                self.use_teleporter(tile);
            }
            if tile >= 0x110 {
                // Program chips (0x110 … 0x112) leave their socket 0xff.
                self.world.carried = (tile - 0xEC) as u16;
                self.set(x, y, 0xFF, false);
                self.request_sound(sound::TAKE);
            }
            if tile != 0 && (tile <= 0x27 || self.world.picked_up) {
                if self.world.picked_up {
                    // Taken from the rail: a piece of rail.
                    self.world.picked_up = false;
                    self.world.carried = 0x1F;
                } else {
                    self.world.carried = tile as u16;
                    self.set(x, y, 0, false);
                    if (16..=19).contains(&tile) {
                        self.load_missile(tile);
                    }
                }
                self.request_sound(sound::TAKE);
            }
            return;
        }
        if tile != 0 && (tile < 0x28 || (0x101..=0x104).contains(&tile)) {
            if tile < 0x28 {
                // Swap with the object on the box.
                self.set(x, y, self.world.carried, false);
                self.world.carried = tile as u16;
                if (16..=19).contains(&tile) {
                    self.load_missile(tile);
                } else {
                    self.player.turret_locked = false;
                    self.player.turret = self.player.dir;
                }
                self.request_sound(sound::TAKE);
            } else {
                self.use_teleporter(tile);
            }
            return;
        }
        let carried = self.world.carried;
        let put = |game: &mut Game, tile: u16| {
            game.set(x, y, tile, false);
            game.world.carried = 0;
            game.request_sound(sound::TAKE);
        };
        match carried {
            // The key of door 61.
            30 if tile == 0x100 => {
                self.request_sound(sound::KEY);
                self.world.switches[4] = false;
                self.world.carried = 0;
            }
            30 if tile == 0 => put(self, 0x1E),
            30 => {}
            // A piece of rail fits into gaps of rails and conveyors.
            31 if (0x8C..=0xA9).contains(&tile) || (0xF7..=0xFD).contains(&tile) => {
                put(self, tile as u16 + 1)
            }
            31 if tile == 0 => put(self, 31),
            31 => {}
            // Program chips go back into sockets.
            36..=38 if tile == 0xFF => put(self, carried - 36 + 0x110),
            36..=38 if tile == 0 => put(self, carried),
            36..=38 => {}
            _ if tile == 0 => {
                put(self, carried);
                self.player.turret_locked = false;
                self.player.turret = self.player.dir;
            }
            _ => {}
        }
    }

    /// Teleporter 0x101 … 0x104 under the player (CODE 9 0x134a).
    fn use_teleporter(&mut self, tile: i32) {
        if self.sounds.on {
            self.play_sound(sound::TELEPORT);
            self.sounds.request = None;
        }
        self.teleport(Some(tile - 0x101));
        self.request_sound(sound::ARRIVAL);
    }

    /// Missiles (16 … 19) load the turret, which then points their way.
    fn load_missile(&mut self, tile: i32) {
        self.player.turret_locked = true;
        self.player.turret = Dir::from_index(tile - 16).unwrap();
    }

    /// Command 4 (CODE 9 0x174c): fire the loaded missile.
    fn fire(&mut self) {
        if !self.player.turret_locked {
            return;
        }
        let (px, py) = self.player_pixel();
        let (vx, vy) = (self.view.0 as i32 * BOX + px, self.view.1 as i32 * BOX + py);
        let fired = match self.player.turret {
            Dir::Left => self.spawn(vx, vy + 16, -2, 0),
            Dir::Up => self.spawn(vx + 16, vy, 0, -2),
            Dir::Right => self.spawn(vx + 30, vy + 16, 2, 0),
            Dir::Down => self.spawn(vx + 16, vy + 30, 0, 2),
        };
        if fired.is_some() {
            self.player.turret_locked = false;
            self.player.turret = self.player.dir;
            self.world.carried = 0;
            self.request_sound(sound::FIRE);
        }
    }

    /// Moving (CODE 9 0x18c2): on arrival near the edge of the view, the
    /// view scrolls by up to 6 boxes sideways or 4 up or down.
    fn player_move(&mut self) {
        if self.player.remaining == 0 {
            return;
        }
        self.player.remaining -= self.player.speed;
        if self.player.remaining != 0 {
            return;
        }
        let dir = self.player.dir;
        let (dx, dy) = dir.delta();
        self.player.x = (self.player.x as i32 + dx) as usize;
        self.player.y = (self.player.y as i32 + dy) as usize;
        let (rx, ry) = self.rel();
        let (vx, vy) = (self.view.0 as i32, self.view.1 as i32);
        let boxes = match dir {
            Dir::Left if rx == 1 && vx != 0 => vx.min(6),
            Dir::Up if ry == 1 && vy != 0 => vy.min(4),
            Dir::Right if rx == 12 && vx != 26 => {
                if vx + rx + 1 + 6 > 39 {
                    38 - (vx + rx)
                } else {
                    6
                }
            }
            Dir::Down if ry == 8 => {
                if vy + ry + 1 + 4 > 23 {
                    22 - (vy + ry)
                } else {
                    4
                }
            }
            _ => return,
        };
        // The original draws the player at the new box first.
        self.stall.frames += 1;
        self.scroll(dir, boxes.max(0) as usize);
    }

    /// Move the view box by box (CODE 9 0x3c6 right, 0x5b2 left, 0x79c
    /// down, 0x986 up); each step updates the drawing state of the
    /// projectiles and takes one frame on screen.
    pub(crate) fn scroll(&mut self, dir: Dir, boxes: usize) {
        if boxes == 0 {
            return;
        }
        if self.stall.scroll.is_none() {
            self.stall.scroll = Some((self.view, dir));
        }
        let (dx, dy) = dir.delta();
        for _ in 0..boxes {
            self.view.0 = (self.view.0 as i32 + dx) as usize;
            self.view.1 = (self.view.1 as i32 + dy) as usize;
            self.update_draw_states();
            self.stall.frames += 1;
        }
    }

    /// Teleporter (CODE 4 0x2054): to the last box (row by row) with tile
    /// 0x105 + n; with `None` back to the player after the magnifier. The
    /// view scrolls box by box until the target is near its center.
    pub(crate) fn teleport(&mut self, n: Option<i32>) {
        for p in self.world.projectiles.iter_mut() {
            p.draw = 0;
        }
        self.stall.frames += 1;
        let target = match n {
            None => Some(self.here()),
            Some(n) => (0..Level::CELLS)
                .rev()
                .find(|&i| self.level.tile(i % Level::WIDTH, i / Level::WIDTH) as i32 == n + 0x105)
                .map(|i| ((i % Level::WIDTH) as i32, (i / Level::WIDTH) as i32)),
        };
        let Some((tx, ty)) = target else {
            return;
        };
        if n.is_some() {
            self.world.player_visible = false;
            self.stall.frames += 1;
        }
        let (vx, vy) = (self.view.0 as i32, self.view.1 as i32);
        let mut dx = tx - vx - 7;
        let mut dy = ty - vy - 5;
        if dx < 0 && vx + dx < 0 {
            dx = -vx;
        } else if dx > 0 && vx + dx > 26 {
            dx = 26 - vx;
        }
        if dy < 0 && vy + dy < 0 {
            dy = -vy;
        } else if dy > 0 && vy + dy > 14 {
            dy = 14 - vy;
        }
        self.stall.scroll = None;
        while dx != 0 || dy != 0 {
            if dx.abs() < dy.abs() {
                if dy < 0 {
                    dy += 1;
                    self.scroll(Dir::Up, 1);
                } else {
                    dy -= 1;
                    self.scroll(Dir::Down, 1);
                }
            } else if dx < 0 {
                dx += 1;
                self.scroll(Dir::Left, 1);
            } else {
                dx -= 1;
                self.scroll(Dir::Right, 1);
            }
        }
        self.stall.scroll = None;
        self.player.x = tx as usize;
        self.player.y = ty as usize;
        self.world.player_visible = true;
        for p in self.world.projectiles.iter_mut() {
            p.draw = if p.draw == 3 || p.draw == 2 { 2 } else { 0 };
        }
    }

    /// A frame of the magnifier (CODE 4 0x1fde): the move keys move the
    /// view.
    pub(crate) fn magnifier_frame(&mut self) {
        let (vx, vy) = self.view;
        match self.player.command {
            0 if vx > 0 => self.scroll(Dir::Left, 1),
            1 if vy > 0 => self.scroll(Dir::Up, 1),
            2 if vx < 26 => self.scroll(Dir::Right, 1),
            3 if vy < 14 => self.scroll(Dir::Down, 1),
            _ => {}
        }
        self.stall.scroll = None;
    }
}
