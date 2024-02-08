//! Structs defined here mirror those in [`crate::map`].
//! The main difference is that they're mostly a 1:1 mapping of the tmx / tsx spec
//! and do not store handle references.
mod map;
mod tileset;

pub use map::*;
pub use tileset::*;