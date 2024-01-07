mod mesh;
pub use mesh::*;

use wgpu::RenderPass;
use crate::GraphicsState;

pub struct G3D {
}

impl G3D {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self, _pass: &mut RenderPass, _graphics_state: &GraphicsState) {

    }
}