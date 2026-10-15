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
pub mod music;
pub mod player;
pub mod progression;
pub mod traps;
pub use model::*;

pub mod simulation_view;
pub use simulation_view::{SimulationView, SimulationViewError, SimulationViewExtent};

pub mod replay;

pub mod world_events;

pub mod characters;
pub(crate) mod creation;
pub mod hub;
pub mod opening;
pub mod statistics;

#[cfg(test)]
mod sound_contract_tests;

#[cfg(test)]
mod music_toggle_tests;

// Explicit native ledger numeric component; no default Scene selection.
pub(crate) mod finite_pc64;
