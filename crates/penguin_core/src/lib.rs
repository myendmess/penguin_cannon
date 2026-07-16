//! Penguin Cannon game logic: biome state machine, orca chase rules, and
//! the tuning table. Rendering-free so the whole crate (and its tests)
//! runs headless on minimal Bevy features.

pub mod chase;
pub mod states;
pub mod tuning;
