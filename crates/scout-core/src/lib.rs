//! Game model and simulation of S.C.Out.
//!
//! This crate does no I/O. The game data is loaded by `scout-data` and the
//! frontend draws the state and feeds in the input.

pub mod game;
pub mod level;
pub mod music;
mod player;
pub mod sound;
pub mod translate;
pub mod world;

pub use game::{Dir, Exit, Game, Key};
pub use level::Level;
pub use sound::Sounds;
pub use world::World;
