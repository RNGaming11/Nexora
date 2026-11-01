//! Horizon's initial simulation primitives, targeting the Minecraft Java Edition version in [`version`].
//!
//! This is not a playable game. Differential checks cover only the operations
//! listed in docs/rust-parity.md; the rest of vanilla remains unimplemented.
#![forbid(unsafe_code)]

pub mod biome;
pub mod carving;
pub mod density;
pub mod geometry;
pub mod jmath;
pub mod no_ai;
pub mod noise;
pub mod paletted;
pub mod position;
pub mod random;
pub mod storage;
pub mod text;
pub mod ticks;

pub mod release;
#[cfg(not(feature = "minecraft-1-21-1"))]
pub mod version;
#[path = "../../../ports/1.21.1/version.rs"]
pub mod version_1_21_1;
#[cfg(feature = "minecraft-1-21-1")]
pub use version_1_21_1 as version;

pub use version::VANILLA_VERSION;
