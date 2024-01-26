//! Module that defines both graphics primitives, and multiple graphics engines that make use of those primitives.
//! The graphics primitives are stored in the domain [`GraphicsState`].
//! The 3D graphics engine is [`G3D`]

mod graphics;
mod texture;
mod state;
mod color;
mod shader;
mod scene;
mod buffer;
pub mod g3d;

pub use graphics::*;
pub use texture::*;
pub use state::*;
pub use color::*;
pub use shader::*;
pub use scene::*;
pub use buffer::*;