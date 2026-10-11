//! Platform-independent Classic simulation. Coordinates are original game pixels.
pub mod activation;
pub mod animation;
mod camera_shake;
pub mod catalog;
pub mod dispatcher;
pub mod economy;
pub mod editor;
pub mod ending;
pub mod enemies;
mod game;
pub mod generation;
pub mod items;
pub mod level;
pub mod model;
pub mod player;
pub mod progression;
pub mod traps;
pub use model::*;

pub mod replay;

pub mod world_events;

pub mod characters;
pub(crate) mod creation;
pub mod hub;
pub mod opening;
pub mod statistics;
