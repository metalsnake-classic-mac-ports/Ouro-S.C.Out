//! Game model and simulation of S.C.Out.
//!
//! This crate does no I/O. The game data is loaded by `scout-data` and the
//! frontend draws the state and feeds in the input.

pub mod game;
pub mod level;
pub mod translate;

pub use game::{Command, Dir, Game};
pub use level::Level;
