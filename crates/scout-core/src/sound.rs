//! Sound effects like the original requests them. The original plays one
//! sound at a time on one channel: the game logic sets a request
//! (`-0x539c`), the main loop starts it at the end of the frame and
//! remembers it (`-0x1afc`) until the channel is quiet again. A few sounds
//! are started directly. Sounds do not change the game.

use crate::Level;
use crate::game::Game;

/// The 'snd ' resources of the original.
pub mod id {
    pub const ACCELERATOR: u8 = 100;
    pub const MOVE: u8 = 101;
    pub const FIRE: u8 = 102;
    /// The key (object 30) put into the lock (tile 0x100).
    pub const KEY: u8 = 103;
    pub const CANNON: u8 = 104;
    /// Seen: explosion 350.
    pub const EXPLOSION: u8 = 105;
    /// Seen: tiles 96 and 99.
    pub const HUM: u8 = 106;
    /// Self destruct; also requested while a big explosion 0x150 is seen.
    pub const BIG_EXPLOSION: u8 = 107;
    pub const CONVEYOR: u8 = 108;
    pub const TELEPORT: u8 = 109;
    /// Seen: doors 62, 67, 72, 77, 82, 87.
    pub const DOOR: u8 = 110;
    pub const ARRIVAL: u8 = 111;
    pub const TAKE: u8 = 112;
    /// Seen: tile 213.
    pub const FIRE_TILE: u8 = 113;
    pub const MAGNIFIER: u8 = 114;
    /// Seen: tiles 105, 114, 123, 132.
    pub const MACHINE: u8 = 116;
    pub const COMPUTER: u8 = 117;
    /// Standing on tile 0x10e.
    pub const SAFE: u8 = 118;
    pub const RAIL: u8 = 120;
    pub const PERFECT: u8 = 121;
    pub const GAME_OVER: u8 = 122;
    pub const ABORTED: u8 = 123;
    pub const TRY_AGAIN: u8 = 124;
}

/// Sound state of a game; the frontend sets `busy` before each frame and
/// takes the sounds to start from `start`.
#[derive(Clone, Debug)]
pub struct Sounds {
    /// Sound effects are on (`-0x6dc0`, Game → Setting).
    pub on: bool,
    /// The channel is still playing.
    pub busy: bool,
    /// Sounds to start now, each one stopping the one before.
    pub start: Vec<u8>,
    pub(crate) request: Option<u8>,
    pub(crate) playing: Option<u8>,
}

impl Default for Sounds {
    fn default() -> Self {
        Self {
            on: true,
            busy: false,
            start: Vec::new(),
            request: None,
            playing: None,
        }
    }
}

impl Game {
    pub(crate) fn request_sound(&mut self, sound: u8) {
        self.sounds.request = Some(sound);
    }

    /// Start a sound at once (CODE 8 0x1ff8 called directly).
    pub(crate) fn play_sound(&mut self, sound: u8) {
        if self.sounds.on {
            self.sounds.start.push(sound);
        }
    }

    /// The end of the main loop's frame (CODE 9 ≈ 0x140): forget a sound
    /// that has ended, add the sounds of what is in view, start the request.
    pub(crate) fn sound_frame(&mut self) {
        if !self.sounds.on {
            return;
        }
        if !self.sounds.busy {
            self.sounds.playing = None;
        }
        self.ambient_sounds();
        if let Some(sound) = self.sounds.request.take() {
            self.sounds.start.push(sound);
            self.sounds.playing = Some(sound);
        }
    }

    /// Sounds of things in view (CODE 9 0x546a), not during the magnifier;
    /// the last box (row by row) with a sound wins.
    fn ambient_sounds(&mut self) {
        if self.player.magnifier != 0 {
            return;
        }
        let (vx, vy) = self.view;
        for y in vy..vy + Level::VIEW_HEIGHT {
            for x in vx..vx + Level::VIEW_WIDTH {
                let (sound, unless) = match self.level.tile(x, y) {
                    96 | 99 => (id::HUM, id::HUM),
                    62 | 67 | 72 | 77 | 82 | 87 => (id::DOOR, id::DOOR),
                    105 | 114 | 123 | 132 => (id::MACHINE, id::MACHINE),
                    213 => (id::FIRE_TILE, id::FIRE_TILE),
                    350 => (id::EXPLOSION, id::EXPLOSION),
                    // compares with 105 but requests 107 (see re/BUGS.md)
                    0x150 => (id::BIG_EXPLOSION, id::EXPLOSION),
                    _ => continue,
                };
                if self.sounds.playing != Some(unless) {
                    self.sounds.request = Some(sound);
                }
            }
        }
    }
}
