//! Game model and simulation of S.C.Out.
//!
//! This crate does no I/O. The game data is loaded by `scout-data` and the
//! frontend draws the state and feeds in the input.

pub mod level;

pub use level::Level;
