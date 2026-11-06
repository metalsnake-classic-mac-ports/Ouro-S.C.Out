//! Tile values (translated numbering, see [`crate::Level`]) that more than
//! one part of the game logic refers to. Values used in one place only stay
//! next to their handler, with a comment.

/// Floor the original puts under things that vanish.
pub const PLAIN_FLOOR: u16 = 13;
/// The explosion animation the original puts on exploding boxes (350 …).
pub const EXPLOSION: u16 = 350;
/// The big explosion animation: the player's death, the destroyed target.
pub const BIG_EXPLOSION: u16 = 0x150;
/// A closed hatch: a checkpoint, and the start box after its animation.
pub const CLOSED_HATCH: u16 = 0x10B;
/// A box on which the player survives explosions and projectiles.
pub const SAFE: u16 = 0x10E;
